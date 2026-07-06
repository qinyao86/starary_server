import {
  Activity,
  Archive,
  ArrowLeft,
  BarChart3,
  Check,
  Cloud,
  Database,
  Globe2,
  HardDrive,
  Languages,
  Library,
  ListChecks,
  Lock,
  LogOut,
  Moon,
  Network,
  Pencil,
  Play,
  Plus,
  Power,
  RefreshCw,
  Search,
  Server,
  Settings,
  ShieldCheck,
  Square,
  Sun,
  Trash2,
  UserPlus,
  Users,
  X
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Badge as UiBadge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import logoImage from "./assets/logo.png";
import {
  api,
  clearStoredToken,
  getStoredToken,
  storeToken,
  type ActivityItem,
  type CurrentUser,
  type LibraryMember,
  type ServerInfo,
  type StorageRoot,
  type TeamLibrary,
  type TeamUser
} from "./api";
import { createTranslator, type Language, type TranslationKey } from "./i18n";
import {
  activity as mockActivity,
  assetBreakdown,
  libraries as designLibraries,
  permissions,
  users as designUsers
} from "./mockData";

type Section =
  | "overview"
  | "service"
  | "libraries"
  | "users"
  | "permissions"
  | "storage"
  | "statistics"
  | "activity"
  | "backups"
  | "settings";

type DeploymentMode = "local" | "cloud";
type ColorTheme = "light" | "dark";
type ApiState = "loading" | "connected" | "unavailable";

const colorThemeStorageKey = "madlibrary_server_admin_theme";
const sectionStorageKey = "madlibrary_server_admin_section";
const rememberedLoginEmailStorageKey = "madlibrary_server_admin_email";

const navItems: Array<{ id: Section; icon: typeof Server; label: TranslationKey }> = [
  { id: "overview", icon: BarChart3, label: "overview" },
  { id: "service", icon: Server, label: "service" },
  { id: "libraries", icon: Library, label: "libraries" },
  { id: "users", icon: Users, label: "users" },
  { id: "permissions", icon: ShieldCheck, label: "permissions" },
  { id: "storage", icon: HardDrive, label: "storage" },
  { id: "statistics", icon: Activity, label: "statistics" },
  { id: "activity", icon: ListChecks, label: "activity" },
  { id: "backups", icon: Archive, label: "backups" },
  { id: "settings", icon: Settings, label: "settings" }
];

function readStoredSection(): Section {
  const stored = localStorage.getItem(sectionStorageKey);
  return navItems.some((item) => item.id === stored) ? (stored as Section) : "overview";
}

export function App() {
  const [language, setLanguage] = useState<Language>("zh");
  const [section, setSection] = useState<Section>(() => readStoredSection());
  const [deploymentMode, setDeploymentMode] = useState<DeploymentMode>("local");
  const [serviceRunning, setServiceRunning] = useState(true);
  const [apiState, setApiState] = useState<ApiState>("loading");
  const [serverInfo, setServerInfo] = useState<ServerInfo | null>(null);
  const [needsOwner, setNeedsOwner] = useState<boolean | null>(null);
  const [token, setToken] = useState<string | null>(() => getStoredToken());
  const [currentUser, setCurrentUser] = useState<CurrentUser | null>(null);
  const [libraries, setLibraries] = useState<TeamLibrary[]>([]);
  const [users, setUsers] = useState<TeamUser[]>([]);
  const [storageRoots, setStorageRoots] = useState<StorageRoot[]>([]);
  const [libraryMembers, setLibraryMembers] = useState<LibraryMember[]>([]);
  const [selectedLibraryId, setSelectedLibraryId] = useState("");
  const [assetTotal, setAssetTotal] = useState(0);
  const [activityItems, setActivityItems] = useState<ActivityItem[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const [previewMode, setPreviewMode] = useState(false);
  const [colorTheme, setColorTheme] = useState<ColorTheme>(() => {
    const stored = localStorage.getItem(colorThemeStorageKey);
    return stored === "light" ? "light" : "dark";
  });
  const t = useMemo(() => createTranslator(language), [language]);

  const previewLibraries: TeamLibrary[] = designLibraries.map((item, index) => ({
    id: `preview-library-${index}`,
    name: item.name,
    description: t(item.policy),
    currentUserRole: index === 0 ? "owner" : "library_manager",
    creatorName: designUsers[index % designUsers.length]?.name ?? "Qin Yao",
    memberNames: designUsers.slice(0, Math.min(3, item.members)).map((user) => user.name),
    assetCount: Number(item.assets.replace(/,/g, "")),
    folderCount: Math.max(0, Math.round(Number(item.assets.replace(/,/g, "")) / 120)),
    tagCount: Math.max(0, Math.round(Number(item.assets.replace(/,/g, "")) / 80)),
    totalSizeBytes: parseStorageSize(item.storage)
  }));
  const previewUsers: TeamUser[] = designUsers.map((item, index) => ({
    id: `preview-user-${index}`,
    email: item.email,
    displayName: item.name,
    globalRole: item.role === "manager" ? "library_manager" : item.role,
    isActive: item.status !== "pending",
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString()
  }));
  const effectiveLibraries = previewMode && libraries.length === 0 ? previewLibraries : libraries;
  const effectiveUsers = previewMode && users.length === 0 ? previewUsers : users;
  const effectiveAssetTotal = previewMode && assetTotal === 0 ? 128430 : assetTotal;
  const effectiveCurrentUser =
    currentUser ??
    (previewMode
      ? {
          id: "preview-owner",
          email: "owner@madlibrary.local",
          displayName: "Qin Yao",
          role: "owner"
        }
      : null);
  const signedIn = previewMode || Boolean(token && currentUser);
  const selectedLibrary = effectiveLibraries.find((item) => item.id === selectedLibraryId) ?? effectiveLibraries[0] ?? null;
  const title = t(navItems.find((item) => item.id === section)?.label ?? "overview");

  const loadPublicState = useCallback(async () => {
    setApiState("loading");
    try {
      const [health, info, setup] = await Promise.all([api.health(), api.serverInfo(), api.setupStatus()]);
      setServerInfo(info);
      setNeedsOwner(setup.needsOwner);
      setServiceRunning(health.status === "ok");
      setDeploymentMode(info.deploymentMode === "local" ? "local" : "cloud");
      setApiState("connected");
    } catch (error) {
      setApiState("unavailable");
      setMessage(error instanceof Error ? error.message : String(error));
    }
  }, []);

  const loadLibraryScopedState = useCallback(async (nextToken: string | null, libraryId: string) => {
    if (!nextToken || !libraryId) {
      setStorageRoots([]);
      setLibraryMembers([]);
      setAssetTotal(0);
      setActivityItems([]);
      return;
    }

    const [roots, members, assets, activity] = await Promise.all([
      api.listStorageRoots(nextToken, libraryId),
      api.listLibraryMembers(nextToken, libraryId),
      api.listAssets(nextToken, libraryId),
      api.listActivity(nextToken, libraryId)
    ]);
    setStorageRoots(roots);
    setLibraryMembers(members);
    setAssetTotal(assets.total);
    setActivityItems(activity.items);
  }, []);

  const loadPrivateState = useCallback(
    async (nextToken = token) => {
      if (!nextToken) return;
      try {
        const [me, nextLibraries, nextUsers] = await Promise.all([
          api.me(nextToken),
          api.listLibraries(nextToken),
          api.listUsers(nextToken)
        ]);
        setCurrentUser(me);
        setLibraries(nextLibraries);
        setUsers(nextUsers);
        const nextLibraryId = nextLibraries.some((item) => item.id === selectedLibraryId)
          ? selectedLibraryId
          : nextLibraries[0]?.id || "";
        setSelectedLibraryId(nextLibraryId);
        await loadLibraryScopedState(nextToken, nextLibraryId);
      } catch (error) {
        clearStoredToken();
        setToken(null);
        setCurrentUser(null);
        setMessage(error instanceof Error ? error.message : String(error));
      }
    },
    [loadLibraryScopedState, selectedLibraryId, token]
  );

  useEffect(() => {
    void loadPublicState();
  }, [loadPublicState]);

  useEffect(() => {
    if (token) {
      void loadPrivateState(token);
    }
  }, [loadPrivateState, token]);

  const refreshAll = async () => {
    await loadPublicState();
    await loadPrivateState(token);
  };

  const selectLibrary = (id: string) => {
    setSelectedLibraryId(id);
    if (token) {
      void loadLibraryScopedState(token, id).catch((error) => {
        setMessage(error instanceof Error ? error.message : String(error));
      });
    }
  };

  const onAuthenticated = (response: { accessToken: string; user: CurrentUser }) => {
    storeToken(response.accessToken);
    setToken(response.accessToken);
    setCurrentUser(response.user);
    setNeedsOwner(false);
    setMessage(null);
  };

  const logout = () => {
    clearStoredToken();
    setToken(null);
    setCurrentUser(null);
    setLibraries([]);
    setUsers([]);
    setStorageRoots([]);
    setActivityItems([]);
  };

  const toggleColorTheme = () => {
    setColorTheme((value) => {
      const next = value === "dark" ? "light" : "dark";
      localStorage.setItem(colorThemeStorageKey, next);
      return next;
    });
  };

  const selectSection = (nextSection: Section) => {
    localStorage.setItem(sectionStorageKey, nextSection);
    setSection(nextSection);
  };

  if (apiState === "loading" && needsOwner === null) {
    return <AuthShell t={t} language={language} setLanguage={setLanguage} title={t("loading")} colorTheme={colorTheme} />;
  }

  if (apiState === "unavailable" && !previewMode) {
    return (
      <AuthShell t={t} language={language} setLanguage={setLanguage} title={t("apiUnavailable")} colorTheme={colorTheme}>
        <div className="auth-note">{message}</div>
        <div className="auth-actions">
          <button className="primary-button" type="button" onClick={() => setPreviewMode(true)}>
            {t("offlinePreview")}
          </button>
        </div>
      </AuthShell>
    );
  }

  if (needsOwner && !previewMode) {
    return (
      <AuthShell t={t} language={language} setLanguage={setLanguage} title={t("setupRequired")} colorTheme={colorTheme}>
        <SetupOwnerForm t={t} onDone={onAuthenticated} />
      </AuthShell>
    );
  }

  if (!signedIn) {
    return (
      <AuthShell t={t} language={language} setLanguage={setLanguage} title={t("login")} colorTheme={colorTheme}>
        <LoginForm t={t} onDone={onAuthenticated} />
      </AuthShell>
    );
  }

  const context: PageContext = {
    t,
    language,
    setLanguage,
    deploymentMode,
    setDeploymentMode,
    serviceRunning,
    serverInfo,
    currentUser: effectiveCurrentUser,
    libraries: effectiveLibraries,
    users: effectiveUsers,
    storageRoots,
    libraryMembers,
    selectedLibrary,
    selectedLibraryId: selectedLibrary?.id ?? selectedLibraryId,
    setSelectedLibraryId: selectLibrary,
    token: token ?? "",
    assetTotal: effectiveAssetTotal,
    activityItems,
    refreshAll,
    setMessage,
    previewMode
  };

  return (
    <div className={`admin-shell ${colorTheme === "dark" ? "dark theme-dark" : "theme-light"}`}>
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <img alt="" src={logoImage} />
          </div>
          <div>
            <div className="brand-title">{t("appName")}</div>
            <div className="brand-subtitle">{t("admin")}</div>
          </div>
        </div>

        <div className="sidebar-status-area">
          <div className="sidebar-server-card">
            <div className="sidebar-server-row">
              <StatusDot label={t(serviceRunning ? "running" : "stopped")} tone={serviceRunning ? "good" : "muted"} />
            </div>
            <div className="sidebar-address">{serverInfo?.serverUrl.replace("http://", "") ?? "127.0.0.1:3789"}</div>
            <div className="sidebar-deployment-label">{deploymentModeLabel(t, deploymentMode)}</div>
          </div>
        </div>

        <nav className="nav-list" aria-label="Server admin navigation">
          {navItems.map((item) => {
            const Icon = item.icon;
            const active = item.id === section;
            return (
              <button
                className={`nav-item${active ? " is-active" : ""}`}
                key={item.id}
                type="button"
                onClick={() => selectSection(item.id)}
              >
                <Icon size={17} />
                <span>{t(item.label)}</span>
              </button>
            );
          })}
        </nav>

        <div className="sidebar-footer">
          <div className="sidebar-quick-actions">
            <button
              className={`sidebar-theme-switch${colorTheme === "dark" ? " is-dark" : ""}`}
              type="button"
              role="switch"
              aria-checked={colorTheme === "dark"}
              aria-label={t(colorTheme === "dark" ? "lightTheme" : "darkTheme")}
              onClick={toggleColorTheme}
            >
              <span className="sidebar-theme-switch-icon is-light"><Sun size={12} /></span>
              <span className="sidebar-theme-switch-icon is-dark"><Moon size={12} /></span>
              <span className="sidebar-theme-switch-thumb">
                {colorTheme === "dark" ? <Moon size={12} /> : <Sun size={12} />}
              </span>
            </button>
            <button className="sidebar-logout-button" type="button" onClick={logout}>
              <LogOut size={15} />
              <span>{t("logout")}</span>
            </button>
          </div>
        </div>
      </aside>

      <main className="main">
        <header className="topbar">
          <div>
            <h1>{title}</h1>
            <div className="crumb">
              {t("deployment")} / {deploymentModeLabel(t, deploymentMode)} / {previewMode ? t("placeholderData") : t("realData")}
            </div>
          </div>
          <div className="topbar-actions">
            <Segmented
              value={deploymentMode}
              options={[
                { value: "local", label: t("local"), icon: Server },
                { value: "cloud", label: t("cloud"), icon: Cloud }
              ]}
              onChange={(value) => setDeploymentMode(value as DeploymentMode)}
            />
            <button className="secondary-button" type="button" onClick={refreshAll}>
              <RefreshCw size={16} />
              <span>{t("refresh")}</span>
            </button>
            <button className="secondary-button" type="button" onClick={logout}>
              <span>{t("logout")}</span>
            </button>
          </div>
        </header>

        {message && (
          <div className="message-bar" role="status" aria-live="polite">
            <span>{message}</span>
            <button type="button" onClick={() => setMessage(null)}>
              x
            </button>
          </div>
        )}

        <section className="content">{renderSection(section, context)}</section>
      </main>
    </div>
  );
}

function renderSection(section: Section, context: PageContext) {
  switch (section) {
    case "service":
      return <PageFrame title={context.t("service")} description={context.t("servicePageHint")}><ServicePage {...context} /></PageFrame>;
    case "libraries":
      return <LibrariesPage {...context} />;
    case "users":
      return <PageFrame title={context.t("users")} description={context.t("usersPageHint")}><UsersPage {...context} /></PageFrame>;
    case "permissions":
      return <PageFrame title={context.t("permissions")} description={context.t("permissionsPageHint")}><PermissionsPage t={context.t} /></PageFrame>;
    case "storage":
      return <PageFrame title={context.t("storage")} description={context.t("storagePageHint")}><StoragePage {...context} /></PageFrame>;
    case "statistics":
      return <PageFrame title={context.t("statistics")} description={context.t("statisticsPageHint")}><StatisticsPage t={context.t} assetTotal={context.assetTotal} users={context.users} libraries={context.libraries} /></PageFrame>;
    case "activity":
      return <PageFrame title={context.t("activity")} description={context.t("activityPageHint")}><ActivityPage t={context.t} activityItems={context.activityItems} /></PageFrame>;
    case "backups":
      return <PageFrame title={context.t("backups")} description={context.t("backupsPageHint")}><BackupsPage t={context.t} deploymentMode={context.deploymentMode} /></PageFrame>;
    case "settings":
      return <PageFrame title={context.t("settings")} description={context.t("settingsPageHint")}><SettingsPage t={context.t} deploymentMode={context.deploymentMode} serverInfo={context.serverInfo} currentUser={context.currentUser} /></PageFrame>;
    default:
      return <PageFrame title={context.t("overview")} description={context.t("overviewPageHint")}><OverviewPage {...context} /></PageFrame>;
  }
}

function PageFrame({
  title,
  titleSlot,
  action,
  children
}: {
  title: string;
  description: string;
  titleSlot?: React.ReactNode;
  action?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="page-frame">
      <div className="page-intro">
        {titleSlot ?? <h2>{title}</h2>}
        {action && <div className="page-intro-action">{action}</div>}
      </div>
      {children}
    </div>
  );
}

function OverviewPage({ t, deploymentMode, serviceRunning, serverInfo, libraries, users, assetTotal, activityItems, storageRoots }: PageContext) {
  const cards = [
    { key: "activeUsers", value: String(users.length), change: t("realData"), tone: "blue" },
    { key: "teamLibraries", value: String(libraries.length), change: t("realData"), tone: "green" },
    { key: "totalAssets", value: String(assetTotal), change: t("realData"), tone: "violet" },
    { key: "sharedRoots", value: String(storageRoots.length), change: t("realData"), tone: "amber" }
  ] as const;

  return (
    <div className="page-grid">
      <div className="metric-grid">
        {cards.map((item) => (
          <MetricCard key={item.key} label={t(item.key)} value={item.value} change={item.change} tone={item.tone} />
        ))}
      </div>

      <Panel title={t("healthChecks")} icon={ShieldCheck} className="span-7">
        <div className="status-grid">
          <HealthRow icon={Server} label={t("serverProcess")} value={t(serviceRunning ? "running" : "stopped")} tone={serviceRunning ? "good" : "muted"} />
          <HealthRow icon={Database} label={t("database")} value={serverInfo?.databaseStatus ?? t("connected")} tone="good" />
          <HealthRow icon={deploymentMode === "local" ? HardDrive : Cloud} label={deploymentMode === "local" ? t("localStorage") : t("objectStorage")} value={storageStatusLabel(t, serverInfo)} tone={serverInfo?.storageWritable ? "good" : "warn"} />
          <HealthRow icon={RefreshCw} label={t("thumbnailQueue")} value={t("plannedNote")} tone="muted" />
          <HealthRow icon={Network} label={t("uploadQueue")} value={t("plannedNote")} tone="muted" />
        </div>
      </Panel>

      <Panel title={t("recentActivity")} icon={Activity} className="span-5">
        <ActivityList t={t} activityItems={activityItems} compact />
      </Panel>

      <Panel title={t("libraries")} icon={Library} className="span-7">
        <DataTable
          emptyLabel={t("noLibraries")}
          columns={[t("libraryName"), t("role"), t("description")]}
          rows={libraries.slice(0, 4).map((item) => [item.name, roleLabel(t, item.currentUserRole ?? "owner"), item.description ?? ""])}
        />
      </Panel>

      <Panel title={t("serverInfo")} icon={HardDrive} className="span-5">
        <InfoStack
          items={[
            [t("serverUrl"), serverInfo?.serverUrl ?? "-"],
            [t("apiVersion"), serverInfo?.apiVersion ?? "-"],
            [t("serverStorageDir"), serverInfo?.storageDir ?? "-"],
            [t("storage"), storageStatusLabel(t, serverInfo)],
            [t("adminAssets"), serverInfo?.adminAvailable ? t("configured") : t("notConfigured")]
          ]}
        />
      </Panel>
    </div>
  );
}

function ServicePage({ t, deploymentMode, serviceRunning, serverInfo }: PageContext) {
  return (
    <div className="page-grid">
      <Panel title={t("serviceControl")} icon={Power} className="span-8">
        <div className="service-control">
          <div className={`service-orb${serviceRunning ? " is-on" : ""}`}>
            {serviceRunning ? <Power size={34} /> : <Square size={34} />}
          </div>
          <div className="service-lines">
            <StatusDot label={t(serviceRunning ? "running" : "stopped")} tone={serviceRunning ? "good" : "muted"} />
            <div className="endpoint-grid">
              <KeyValue label={t("serverUrl")} value={serverInfo?.serverUrl ?? "http://127.0.0.1:3789"} />
              <KeyValue label={t(deploymentMode === "local" ? "lanAddress" : "publicUrl")} value={deploymentMode === "local" ? "http://192.168.3.20:3789" : "https://team.example.com"} />
              <KeyValue label={t("apiVersion")} value={serverInfo?.apiVersion ?? "v1"} />
              <KeyValue label={t("mode")} value={deploymentModeLabel(t, deploymentMode)} />
            </div>
          </div>
          <div className="button-row vertical">
            <button className="primary-button is-disabled" type="button">
              {serviceRunning ? <Power size={16} /> : <Play size={16} />}
              <span>{t(serviceRunning ? "stopService" : "startService")}</span>
            </button>
            <button className="secondary-button is-disabled" type="button">
              <RefreshCw size={16} />
              <span>{t("restart")}</span>
            </button>
          </div>
        </div>
      </Panel>

      <Panel title={t("database")} icon={Database} className="span-4">
        <InfoStack
          items={[
            [t("postgresql"), deploymentMode === "local" ? t("running") : t("managedByDeployment")],
            [t("status"), serverInfo?.databaseStatus ?? t("connected")],
            [t("storageUsed"), t("plannedNote")]
          ]}
        />
      </Panel>

      <Panel title={t("healthChecks")} icon={ShieldCheck} className="span-12">
        <div className="health-line">
          <HealthRow icon={Server} label={t("serverProcess")} value={t("healthy")} tone="good" />
          <HealthRow icon={Database} label={t("database")} value={t("connected")} tone="good" />
          <HealthRow icon={HardDrive} label={t("storage")} value={storageStatusLabel(t, serverInfo)} tone={serverInfo?.storageWritable ? "good" : "warn"} />
          <HealthRow icon={Globe2} label={t("publicUrl")} value={deploymentMode === "cloud" ? t("notConfigured") : t("disabled")} tone="muted" />
        </div>
      </Panel>
    </div>
  );
}

function LibrariesPage({
  t,
  token,
  libraries,
  users,
  libraryMembers,
  selectedLibraryId,
  setSelectedLibraryId,
  refreshAll,
  setMessage
}: PageContext) {
  const [viewLibraryId, setViewLibraryId] = useState<string | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [editingLibraryId, setEditingLibraryId] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [editName, setEditName] = useState("");
  const [editDescription, setEditDescription] = useState("");
  const [memberUserId, setMemberUserId] = useState("");
  const [memberRole, setMemberRole] = useState("viewer");
  const [memberDialogOpen, setMemberDialogOpen] = useState(false);
  const activeLibrary = libraries.find((item) => item.id === viewLibraryId) ?? null;
  const availableUsers = useMemo(
    () => users.filter((user) => !libraryMembers.some((member) => member.userId === user.id)),
    [libraryMembers, users]
  );

  useEffect(() => {
    if (!memberUserId && availableUsers.length > 0) {
      setMemberUserId(availableUsers[0].id);
    } else if (memberUserId && !availableUsers.some((user) => user.id === memberUserId)) {
      setMemberUserId(availableUsers[0]?.id ?? "");
    }
  }, [availableUsers, memberUserId]);

  const openMemberDialog = () => {
    setMemberUserId(availableUsers[0]?.id ?? "");
    setMemberRole("viewer");
    setMemberDialogOpen(true);
  };

  const closeMemberDialog = () => {
    setMemberDialogOpen(false);
  };

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!name.trim()) {
      setMessage(t("formRequiredHint"));
      return;
    }
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      const library = await api.createLibrary(token, { name: name.trim(), description: description.trim() || undefined });
      setName("");
      setDescription("");
      setShowCreate(false);
      setViewLibraryId(library.id);
      setSelectedLibraryId(library.id);
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const openCreateLibraryDialog = () => {
    cancelEditLibrary();
    setName("");
    setDescription("");
    setShowCreate(true);
  };

  const closeCreateLibraryDialog = () => {
    setShowCreate(false);
    setName("");
    setDescription("");
  };

  const openLibrary = (libraryId: string) => {
    setViewLibraryId(libraryId);
    setSelectedLibraryId(libraryId);
  };

  const startEditLibrary = (library: TeamLibrary) => {
    setShowCreate(false);
    setEditingLibraryId(library.id);
    setEditName(library.name);
    setEditDescription(library.description ?? "");
  };

  const cancelEditLibrary = () => {
    setEditingLibraryId(null);
    setEditName("");
    setEditDescription("");
  };

  const submitEditLibrary = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!editingLibraryId || !editName.trim()) {
      setMessage(t("formRequiredHint"));
      return;
    }
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.updateLibrary(token, editingLibraryId, {
        name: editName.trim(),
      description: editDescription.trim() || undefined
    });
      cancelEditLibrary();
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const deleteLibrary = async (library: TeamLibrary) => {
    if (!window.confirm(t("deleteLibraryConfirm"))) return;
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.deleteLibrary(token, library.id);
      if (viewLibraryId === library.id) setViewLibraryId(null);
      if (selectedLibraryId === library.id) setSelectedLibraryId("");
      if (editingLibraryId === library.id) cancelEditLibrary();
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const submitMember = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!selectedLibraryId || !memberUserId) {
      setMessage(t("formRequiredHint"));
      return;
    }
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.upsertLibraryMember(token, selectedLibraryId, memberUserId, { role: memberRole });
      closeMemberDialog();
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const removeMember = async (member: LibraryMember) => {
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.removeLibraryMember(token, member.libraryId, member.userId);
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  if (activeLibrary) {
    return (
      <PageFrame
        title={t("libraries")}
        description={t("libraryPageHint")}
        titleSlot={
          <button className="page-back-button" type="button" onClick={() => setViewLibraryId(null)}>
            <ArrowLeft size={16} />
            <span>{t("backToLibraries")}</span>
          </button>
        }
      >
      <div className="library-page">
        <MemberDialog
          open={memberDialogOpen}
          t={t}
          users={availableUsers}
          memberUserId={memberUserId}
          memberRole={memberRole}
          onClose={closeMemberDialog}
          onRoleChange={setMemberRole}
          onSubmit={submitMember}
          onUserChange={setMemberUserId}
        />

        <Card className="library-detail-hero">
          <CardHeader className="p-0">
            <CardDescription>{t("libraryDetails")}</CardDescription>
            <CardTitle className="text-[22px]">{activeLibrary.name}</CardTitle>
            <CardDescription>{activeLibrary.description || t("noDescription")}</CardDescription>
          </CardHeader>
          <CardContent className="library-detail-stats p-0">
            <LibraryDetailStat label={t("totalSize")} value={formatBytes(activeLibrary.totalSizeBytes)} />
            <LibraryDetailStat label={t("members")} value={formatCount(libraryMembers.length)} />
            <LibraryDetailStat label={t("assets")} value={formatCount(activeLibrary.assetCount)} />
            <LibraryDetailStat label={t("tags")} value={formatCount(activeLibrary.tagCount)} />
            <LibraryDetailStat label={t("creator")} value={activeLibrary.creatorName || "-"} />
            <LibraryDetailStat label={t("role")} value={roleLabel(t, activeLibrary.currentUserRole ?? "owner")} />
          </CardContent>
        </Card>

        <div className="page-grid">
          <Panel
            title={t("libraryMembers")}
            icon={Users}
            className="span-12"
            action={
              <div className="panel-actions">
                <UiBadge className="control-badge" variant="secondary">{libraryMembers.length ? t("realData") : t("empty")}</UiBadge>
                <Button className="panel-action-button" size="sm" type="button" onClick={openMemberDialog} disabled={availableUsers.length === 0}>
                  <UserPlus size={15} />
                  <span>{t("addMember")}</span>
                </Button>
              </div>
            }
          >
            {libraryMembers.length === 0 ? (
              <div className="placeholder-box">{t("noMembers")}</div>
            ) : (
              <div className="member-list">
                {libraryMembers.map((member) => (
                  <div className="member-row" key={member.userId}>
                    <div>
                      <strong>{member.displayName}</strong>
                      <span>{member.email}</span>
                    </div>
                    <UiBadge className="member-role-badge" variant="secondary">{roleLabel(t, member.role)}</UiBadge>
                    <Button className="member-action-button" size="sm" type="button" variant="outline" onClick={() => void removeMember(member)}>
                      <Trash2 size={15} />
                      <span>{t("remove")}</span>
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </Panel>
        </div>
      </div>
      </PageFrame>
    );
  }

  return (
    <PageFrame
      title={t("libraries")}
      description={t("libraryPageHint")}
     
      action={
        <Button type="button" onClick={openCreateLibraryDialog}>
          <Plus size={16} />
          <span>{t("createLibrary")}</span>
        </Button>
      }
    >
    <div className="library-page">
      <LibraryDialog
        open={showCreate}
        title={t("createLibrary")}
        hint={t("libraryPageHint")}
        name={name}
        description={description}
        submitLabel={t("submit")}
        t={t}
        onClose={closeCreateLibraryDialog}
        onDescriptionChange={setDescription}
        onNameChange={setName}
        onSubmit={submit}
      />

      <LibraryDialog
        open={Boolean(editingLibraryId)}
        title={t("updateLibrary")}
        hint={t("libraryPageHint")}
        name={editName}
        description={editDescription}
        submitLabel={t("submit")}
        t={t}
        onClose={cancelEditLibrary}
        onDescriptionChange={setEditDescription}
        onNameChange={setEditName}
        onSubmit={submitEditLibrary}
      />

      {libraries.length === 0 ? (
        <div className="placeholder-box">{t("noLibraries")}</div>
      ) : (
        <div className="library-card-grid">
          {libraries.map((library) => (
            <Card className="library-card" key={library.id}>
              <div className="library-card-top">
                <div className="library-card-heading">
                  <div className="library-card-title">{library.name}</div>
                  <div className="library-card-description">{library.description || t("noDescription")}</div>
                </div>
                <div className="library-card-tools">
                  <button className="library-icon-button" type="button" aria-label={t("editLibrary")} onClick={() => startEditLibrary(library)}>
                    <Pencil size={15} />
                  </button>
                  <button className="library-icon-button is-danger" type="button" aria-label={t("deleteLibrary")} onClick={() => void deleteLibrary(library)}>
                    <Trash2 size={15} />
                  </button>
                </div>
              </div>
              <div className="library-card-stats">
                <LibraryStat label={t("totalSize")} value={formatBytes(library.totalSizeBytes)} />
                <LibraryStat label={t("membersLabel")} value={formatCount(library.memberNames?.length)} />
                <LibraryStat label={t("assets")} value={formatCount(library.assetCount)} />
                <LibraryStat label={t("tags")} value={formatCount(library.tagCount)} />
              </div>
              <div className="library-card-footer">
                <div className="library-card-members" title={library.creatorName ?? ""}>
                  <span>{t("creator")}</span>
                  <strong>{library.creatorName || "-"}</strong>
                </div>
                <Button size="sm" type="button" onClick={() => openLibrary(library.id)}>
                  <Library size={15} />
                  <span>{t("openLibrary")}</span>
                </Button>
              </div>
            </Card>
          ))}
        </div>
      )}
    </div>
    </PageFrame>
  );
}

function LibraryDialog({
  open,
  title,
  hint,
  name,
  description,
  submitLabel,
  t,
  onClose,
  onDescriptionChange,
  onNameChange,
  onSubmit
}: TranslatorContext & {
  open: boolean;
  title: string;
  hint: string;
  name: string;
  description: string;
  submitLabel: string;
  onClose: () => void;
  onDescriptionChange: (value: string) => void;
  onNameChange: (value: string) => void;
  onSubmit: (event: React.FormEvent) => void | Promise<void>;
}) {
  useEffect(() => {
    if (!open) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose, open]);

  if (!open) return null;

  return (
    <div
      className="dialog-backdrop"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <section className="dialog-panel library-dialog" role="dialog" aria-modal="true" aria-labelledby="library-dialog-title">
        <div className="dialog-header">
          <div>
            <h2 className="dialog-title" id="library-dialog-title">{title}</h2>
            <p className="dialog-subtitle">{hint}</p>
          </div>
          <Button className="dialog-close" type="button" variant="ghost" size="icon" aria-label={t("cancel")} onClick={onClose}>
            <X size={16} />
          </Button>
        </div>
        <form className="dialog-form" onSubmit={onSubmit}>
          <div className="dialog-body">
            <TextField autoFocus required label={t("name")} value={name} onChange={onNameChange} />
            <TextField label={t("description")} value={description} onChange={onDescriptionChange} />
          </div>
          <div className="dialog-footer">
            <Button type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
            <Button type="submit">{submitLabel}</Button>
          </div>
        </form>
      </section>
    </div>
  );
}

function MemberDialog({
  open,
  t,
  users,
  memberUserId,
  memberRole,
  onClose,
  onRoleChange,
  onSubmit,
  onUserChange
}: TranslatorContext & {
  open: boolean;
  users: TeamUser[];
  memberUserId: string;
  memberRole: string;
  onClose: () => void;
  onRoleChange: (value: string) => void;
  onSubmit: (event: React.FormEvent) => void | Promise<void>;
  onUserChange: (value: string) => void;
}) {
  useEffect(() => {
    if (!open) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose, open]);

  if (!open) return null;

  return (
    <div
      className="dialog-backdrop"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <section className="dialog-panel member-dialog" role="dialog" aria-modal="true" aria-labelledby="member-dialog-title">
        <div className="dialog-header">
          <div>
            <h2 className="dialog-title" id="member-dialog-title">{t("addMember")}</h2>
            <p className="dialog-subtitle">{t("addMemberHint")}</p>
          </div>
          <Button className="dialog-close" type="button" variant="ghost" size="icon" aria-label={t("cancel")} onClick={onClose}>
            <X size={16} />
          </Button>
        </div>
        <form className="dialog-form" onSubmit={onSubmit}>
          <div className="dialog-body">
            {users.length === 0 ? (
              <div className="placeholder-box">{t("noAvailableUsers")}</div>
            ) : (
              <>
                <label className="field">
                  <span>{t("users")}</span>
                  <select value={memberUserId} onChange={(event) => onUserChange(event.target.value)}>
                    {users.map((item) => (
                      <option key={item.id} value={item.id}>{item.displayName} ({item.email})</option>
                    ))}
                  </select>
                </label>
                <label className="field">
                  <span>{t("role")}</span>
                  <select value={memberRole} onChange={(event) => onRoleChange(event.target.value)}>
                    <option value="library_manager">{t("manager")}</option>
                    <option value="editor">{t("editor")}</option>
                    <option value="viewer">{t("viewer")}</option>
                  </select>
                </label>
              </>
            )}
          </div>
          <div className="dialog-footer">
            <Button type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
            <Button type="submit" disabled={!memberUserId || users.length === 0}>{t("submit")}</Button>
          </div>
        </form>
      </section>
    </div>
  );
}

function LibraryStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="library-card-stat">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function LibraryDetailStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="library-detail-stat">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function UserDialog({
  canAssignOwner,
  displayName,
  editingUser,
  email,
  isActive,
  open,
  password,
  role,
  t,
  onClose,
  onDisplayNameChange,
  onEmailChange,
  onIsActiveChange,
  onPasswordChange,
  onRoleChange,
  onSubmit
}: TranslatorContext & {
  canAssignOwner: boolean;
  displayName: string;
  editingUser: TeamUser | null;
  email: string;
  isActive: boolean;
  open: boolean;
  password: string;
  role: string;
  onClose: () => void;
  onDisplayNameChange: (value: string) => void;
  onEmailChange: (value: string) => void;
  onIsActiveChange: (value: boolean) => void;
  onPasswordChange: (value: string) => void;
  onRoleChange: (value: string) => void;
  onSubmit: (event: React.FormEvent) => void | Promise<void>;
}) {
  useEffect(() => {
    if (!open) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose, open]);

  if (!open) return null;

  const title = editingUser ? t("editUser") : t("createUser");

  return (
    <div
      className="dialog-backdrop"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <section className="dialog-panel user-dialog" role="dialog" aria-modal="true" aria-labelledby="user-dialog-title">
        <div className="dialog-header">
          <div>
            <h2 className="dialog-title" id="user-dialog-title">{title}</h2>
            <p className="dialog-subtitle">{t("usersPageHint")}</p>
          </div>
          <Button className="dialog-close" type="button" variant="ghost" size="icon" aria-label={t("cancel")} onClick={onClose}>
            <X size={16} />
          </Button>
        </div>
        <form className="dialog-form" onSubmit={onSubmit}>
          <div className="dialog-body">
            {editingUser ? (
              <div className="readonly-field">
                <span>{t("email")}</span>
                <strong>{email}</strong>
              </div>
            ) : (
              <TextField autoFocus required label={t("email")} value={email} onChange={onEmailChange} />
            )}
            <TextField label={t("displayName")} value={displayName} onChange={onDisplayNameChange} />
            <label className="field">
              <span>{t("role")}</span>
              <select value={role} onChange={(event) => onRoleChange(event.target.value)}>
                {canAssignOwner && <option value="owner">{t("owner")}</option>}
                <option value="admin">{t("adminRole")}</option>
                <option value="library_manager">{t("manager")}</option>
                <option value="editor">{t("editor")}</option>
                <option value="viewer">{t("viewer")}</option>
              </select>
            </label>
            {editingUser && (
              <label className="field">
                <span>{t("status")}</span>
                <select value={isActive ? "enabled" : "disabled"} onChange={(event) => onIsActiveChange(event.target.value === "enabled")}>
                  <option value="enabled">{t("enabled")}</option>
                  <option value="disabled">{t("disabled")}</option>
                </select>
              </label>
            )}
            <TextField
              label={editingUser ? t("newPassword") : t("password")}
              value={password}
              onChange={onPasswordChange}
              type="password"
              required={!editingUser}
              placeholder={editingUser ? t("passwordOptional") : undefined}
            />
            {editingUser && <p className="dialog-hint">{t("passwordOptional")}</p>}
          </div>
          <div className="dialog-footer">
            <Button type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
            <Button type="submit">{t("submit")}</Button>
          </div>
        </form>
      </section>
    </div>
  );
}

function UsersPage({ t, token, users, currentUser, refreshAll, setMessage }: PageContext) {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingUser, setEditingUser] = useState<TeamUser | null>(null);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [role, setRole] = useState("viewer");
  const [isActive, setIsActive] = useState(true);
  const [query, setQuery] = useState("");

  const canAssignOwner = currentUser?.role === "owner";
  const filteredUsers = useMemo(() => {
    const value = query.trim().toLocaleLowerCase();
    if (!value) return users;
    return users.filter((item) =>
      [item.displayName, item.email, roleLabel(t, item.globalRole), item.isActive ? t("enabled") : t("disabled")]
        .join(" ")
        .toLocaleLowerCase()
        .includes(value)
    );
  }, [query, t, users]);

  const openCreateDialog = () => {
    setEditingUser(null);
    setEmail("");
    setPassword("");
    setDisplayName("");
    setRole("viewer");
    setIsActive(true);
    setDialogOpen(true);
  };

  const openEditDialog = (user: TeamUser) => {
    setEditingUser(user);
    setEmail(user.email);
    setPassword("");
    setDisplayName(user.displayName);
    setRole(user.globalRole);
    setIsActive(user.isActive);
    setDialogOpen(true);
  };

  const closeDialog = () => {
    setDialogOpen(false);
  };

  const submitUser = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!editingUser && (!email.trim() || !password.trim())) {
      setMessage(t("formRequiredHint"));
      return;
    }
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      if (editingUser) {
        await api.updateUser(token, editingUser.id, {
          displayName: displayName.trim() || undefined,
          role,
          isActive,
          password: password.trim() || undefined
        });
      } else {
        await api.createUser(token, {
          email: email.trim(),
          password,
          displayName: displayName.trim() || undefined,
          role
        });
      }
      closeDialog();
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const toggleUserActive = async (user: TeamUser) => {
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.updateUser(token, user.id, {
        displayName: user.displayName,
        role: user.globalRole,
        isActive: !user.isActive
      });
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <div className="page-grid">
      <UserDialog
        canAssignOwner={canAssignOwner}
        displayName={displayName}
        editingUser={editingUser}
        email={email}
        isActive={isActive}
        open={dialogOpen}
        password={password}
        role={role}
        t={t}
        onClose={closeDialog}
        onDisplayNameChange={setDisplayName}
        onEmailChange={setEmail}
        onIsActiveChange={setIsActive}
        onPasswordChange={setPassword}
        onRoleChange={setRole}
        onSubmit={submitUser}
      />
      <Panel
        title={t("users")}
        icon={Users}
        className="span-12"
        action={
          <Button className="panel-action-button" size="sm" type="button" onClick={openCreateDialog}>
            <UserPlus size={15} />
            <span>{t("createUser")}</span>
          </Button>
        }
      >
        <div className="toolbar-strip">
          <div className="search-box">
            <Search size={16} />
            <input value={query} placeholder={t("search")} onChange={(event) => setQuery(event.target.value)} />
          </div>
          <StatusDot label={`${t("seatUsage")} ${users.length}`} tone="good" />
        </div>
        <div className="table-wrap">
          <table className="data-table">
            <thead>
              <tr>
                <th>{t("name")}</th>
                <th>{t("email")}</th>
                <th>{t("role")}</th>
                <th>{t("status")}</th>
                <th>{t("action")}</th>
              </tr>
            </thead>
            <tbody>
              {filteredUsers.length === 0 ? (
                <tr>
                  <td colSpan={5}>{t("empty")}</td>
                </tr>
              ) : (
                filteredUsers.map((item) => {
                  const isOwnerUser = item.globalRole === "owner";
                  const canEditUser = canAssignOwner || !isOwnerUser;
                  return (
                    <tr key={item.id}>
                      <td>{item.displayName}</td>
                      <td>{item.email}</td>
                      <td>{roleLabel(t, item.globalRole)}</td>
                      <td><span className={`status-pill${item.isActive ? " is-on" : " is-off"}`}>{item.isActive ? t("enabled") : t("disabled")}</span></td>
                      <td>
                        <div className="table-actions">
                          <Button className="table-action-button" size="sm" type="button" variant="outline" onClick={() => openEditDialog(item)} disabled={!canEditUser}>
                            <Pencil size={14} />
                            <span>{t("edit")}</span>
                          </Button>
                          <Button className="table-action-button" size="sm" type="button" variant="outline" onClick={() => void toggleUserActive(item)} disabled={!canEditUser}>
                            <Power size={14} />
                            <span>{item.isActive ? t("deactivate") : t("activate")}</span>
                          </Button>
                        </div>
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </Panel>
    </div>
  );
}

function PermissionsPage({ t }: TranslatorContext) {
  const roles = ["owner", "adminRole", "manager", "editor", "viewer"] as const;
  return (
    <div className="page-grid">
      <Panel title={t("permissionMatrix")} icon={ShieldCheck} className="span-12" action={<Badge>{t("placeholderData")}</Badge>}>
        <table className="matrix">
          <thead>
            <tr>
              <th>{t("action")}</th>
              {roles.map((role) => (
                <th key={role}>{t(role)}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {permissions.map((row) => (
              <tr key={row.action}>
                <td>{t(row.action)}</td>
                <td><CheckCell checked={row.owner} /></td>
                <td><CheckCell checked={row.admin} /></td>
                <td><CheckCell checked={row.manager} /></td>
                <td><CheckCell checked={row.editor} /></td>
                <td><CheckCell checked={row.viewer} /></td>
              </tr>
            ))}
          </tbody>
        </table>
      </Panel>
    </div>
  );
}

function StorageRootDialog({
  canonicalUri,
  editingRoot,
  enabled,
  kind,
  libraries,
  macosAliases,
  macosSmbUrl,
  name,
  open,
  selectedLibraryId,
  t,
  windowsAliases,
  windowsUncPath,
  onCanonicalUriChange,
  onClose,
  onEnabledChange,
  onKindChange,
  onLibraryChange,
  onMacosAliasesChange,
  onMacosSmbUrlChange,
  onNameChange,
  onSubmit,
  onWindowsAliasesChange,
  onWindowsUncPathChange
}: TranslatorContext & {
  canonicalUri: string;
  editingRoot: StorageRoot | null;
  enabled: boolean;
  kind: string;
  libraries: TeamLibrary[];
  macosAliases: string;
  macosSmbUrl: string;
  name: string;
  open: boolean;
  selectedLibraryId: string;
  windowsAliases: string;
  windowsUncPath: string;
  onCanonicalUriChange: (value: string) => void;
  onClose: () => void;
  onEnabledChange: (value: boolean) => void;
  onKindChange: (value: string) => void;
  onLibraryChange: (value: string) => void;
  onMacosAliasesChange: (value: string) => void;
  onMacosSmbUrlChange: (value: string) => void;
  onNameChange: (value: string) => void;
  onSubmit: (event: React.FormEvent) => void | Promise<void>;
  onWindowsAliasesChange: (value: string) => void;
  onWindowsUncPathChange: (value: string) => void;
}) {
  useEffect(() => {
    if (!open) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose, open]);

  if (!open) return null;

  return (
    <div
      className="dialog-backdrop"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <section className="dialog-panel storage-dialog" role="dialog" aria-modal="true" aria-labelledby="storage-dialog-title">
        <div className="dialog-header">
          <div>
            <h2 className="dialog-title" id="storage-dialog-title">{editingRoot ? t("editStorageRoot") : t("createStorageRoot")}</h2>
            <p className="dialog-subtitle">{t("storagePageHint")}</p>
          </div>
          <Button className="dialog-close" type="button" variant="ghost" size="icon" aria-label={t("cancel")} onClick={onClose}>
            <X size={16} />
          </Button>
        </div>
        <form className="dialog-form" onSubmit={onSubmit}>
          <div className="dialog-body dialog-grid">
            <label className="field">
              <span>{t("selectLibrary")}</span>
              <select value={selectedLibraryId} onChange={(event) => onLibraryChange(event.target.value)} disabled={Boolean(editingRoot)}>
                {libraries.map((item) => (
                  <option key={item.id} value={item.id}>{item.name}</option>
                ))}
              </select>
            </label>
            <TextField autoFocus required label={t("name")} value={name} onChange={onNameChange} />
            <label className="field">
              <span>{t("kind")}</span>
              <select value={kind} onChange={(event) => onKindChange(event.target.value)}>
                <option value="smb">SMB</option>
                <option value="server_filesystem">Filesystem</option>
                <option value="s3">S3</option>
              </select>
            </label>
            <label className="field">
              <span>{t("status")}</span>
              <select value={enabled ? "enabled" : "disabled"} onChange={(event) => onEnabledChange(event.target.value === "enabled")}>
                <option value="enabled">{t("enabled")}</option>
                <option value="disabled">{t("disabled")}</option>
              </select>
            </label>
            <div className="span-field">
              <TextField required label={t("canonicalUri")} value={canonicalUri} onChange={onCanonicalUriChange} />
            </div>
            <TextField label={t("windowsUncPath")} value={windowsUncPath} onChange={onWindowsUncPathChange} />
            <TextField label={t("windowsAliases")} value={windowsAliases} onChange={onWindowsAliasesChange} placeholder={t("commaSeparated")} />
            <TextField label={t("macosSmbUrl")} value={macosSmbUrl} onChange={onMacosSmbUrlChange} />
            <TextField label={t("macosAliases")} value={macosAliases} onChange={onMacosAliasesChange} placeholder={t("commaSeparated")} />
          </div>
          <div className="dialog-footer">
            <Button type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
            <Button type="submit">{t("submit")}</Button>
          </div>
        </form>
      </section>
    </div>
  );
}

function storageRootPayloadFromRecord(root: StorageRoot, enabled = root.enabled) {
  return {
    name: root.name,
    kind: root.kind,
    canonicalUri: root.canonicalUri,
    windowsUncPath: root.windowsUncPath ?? undefined,
    windowsMappedDriveAliases: root.windowsMappedDriveAliases,
    macosSmbUrl: root.macosSmbUrl ?? undefined,
    macosMountAliases: root.macosMountAliases,
    enabled
  };
}

function StoragePage({ t, token, libraries, selectedLibraryId, setSelectedLibraryId, storageRoots, deploymentMode, refreshAll, setMessage }: PageContext) {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingRoot, setEditingRoot] = useState<StorageRoot | null>(null);
  const [name, setName] = useState("");
  const [kind, setKind] = useState("smb");
  const [canonicalUri, setCanonicalUri] = useState("");
  const [windowsUncPath, setWindowsUncPath] = useState("");
  const [windowsAliases, setWindowsAliases] = useState("");
  const [macosSmbUrl, setMacosSmbUrl] = useState("");
  const [macosAliases, setMacosAliases] = useState("");
  const [rootEnabled, setRootEnabled] = useState(true);

  const resetRootForm = () => {
    setName("");
    setKind("smb");
    setCanonicalUri("");
    setWindowsUncPath("");
    setWindowsAliases("");
    setMacosSmbUrl("");
    setMacosAliases("");
    setRootEnabled(true);
  };

  const openCreateDialog = () => {
    setEditingRoot(null);
    resetRootForm();
    setDialogOpen(true);
  };

  const openEditDialog = (root: StorageRoot) => {
    setEditingRoot(root);
    setSelectedLibraryId(root.libraryId);
    setName(root.name);
    setKind(root.kind);
    setCanonicalUri(root.canonicalUri);
    setWindowsUncPath(root.windowsUncPath ?? "");
    setWindowsAliases(root.windowsMappedDriveAliases.join(", "));
    setMacosSmbUrl(root.macosSmbUrl ?? "");
    setMacosAliases(root.macosMountAliases.join(", "));
    setRootEnabled(root.enabled);
    setDialogOpen(true);
  };

  const closeDialog = () => {
    setDialogOpen(false);
  };

  const submitRoot = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!selectedLibraryId || !name.trim() || !canonicalUri.trim()) {
      setMessage(t("formRequiredHint"));
      return;
    }
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      const payload = {
        name: name.trim(),
        kind,
        canonicalUri: canonicalUri.trim(),
        windowsUncPath: windowsUncPath.trim() || undefined,
        windowsMappedDriveAliases: splitList(windowsAliases),
        macosSmbUrl: macosSmbUrl.trim() || undefined,
        macosMountAliases: splitList(macosAliases),
        enabled: rootEnabled
      };
      if (editingRoot) {
        await api.updateStorageRoot(token, editingRoot.id, payload);
      } else {
        await api.createStorageRoot(token, {
          libraryId: selectedLibraryId,
          ...payload
        });
      }
      closeDialog();
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const toggleStorageRoot = async (root: StorageRoot) => {
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.updateStorageRoot(token, root.id, storageRootPayloadFromRecord(root, !root.enabled));
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const deleteStorageRoot = async (root: StorageRoot) => {
    if (!window.confirm(t("deleteStorageRootConfirm"))) return;
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.deleteStorageRoot(token, root.id);
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <div className="page-grid">
      <StorageRootDialog
        canonicalUri={canonicalUri}
        enabled={rootEnabled}
        editingRoot={editingRoot}
        kind={kind}
        libraries={libraries}
        macosAliases={macosAliases}
        macosSmbUrl={macosSmbUrl}
        name={name}
        open={dialogOpen}
        selectedLibraryId={selectedLibraryId}
        t={t}
        windowsAliases={windowsAliases}
        windowsUncPath={windowsUncPath}
        onCanonicalUriChange={setCanonicalUri}
        onClose={closeDialog}
        onEnabledChange={setRootEnabled}
        onKindChange={setKind}
        onLibraryChange={setSelectedLibraryId}
        onMacosAliasesChange={setMacosAliases}
        onMacosSmbUrlChange={setMacosSmbUrl}
        onNameChange={setName}
        onSubmit={submitRoot}
        onWindowsAliasesChange={setWindowsAliases}
        onWindowsUncPathChange={setWindowsUncPath}
      />
      <Panel title={t("storage")} icon={HardDrive} className="span-4">
        <InfoStack
          items={[
            [t("provider"), deploymentMode === "local" ? "Filesystem" : "S3 / MinIO"],
            [t("objectStorage"), deploymentMode === "cloud" ? t("notConfigured") : t("disabled")],
            [t("pathResolver"), t("enabled")]
          ]}
        />
      </Panel>
      <Panel
        title={t("sharedRoots")}
        icon={Network}
        className="span-8"
        action={
          <Button className="panel-action-button" size="sm" type="button" onClick={openCreateDialog} disabled={libraries.length === 0}>
            <Plus size={15} />
            <span>{t("createStorageRoot")}</span>
          </Button>
        }
      >
        <div className="toolbar-strip">
          <label className="field compact-select-field">
            <span>{t("selectLibrary")}</span>
            <select value={selectedLibraryId} onChange={(event) => setSelectedLibraryId(event.target.value)}>
              {libraries.map((item) => (
                <option key={item.id} value={item.id}>{item.name}</option>
              ))}
            </select>
          </label>
          <StatusDot label={storageRoots.length ? t("realData") : t("empty")} tone={storageRoots.length ? "good" : "muted"} />
        </div>
        <div className="table-wrap">
          <table className="data-table">
            <thead>
              <tr>
                <th>{t("name")}</th>
                <th>{t("provider")}</th>
                <th>{t("canonicalUri")}</th>
                <th>{t("status")}</th>
                <th>{t("action")}</th>
              </tr>
            </thead>
            <tbody>
              {storageRoots.length === 0 ? (
                <tr>
                  <td colSpan={5}>{t("noSharedRoots")}</td>
                </tr>
              ) : (
                storageRoots.map((item) => (
                  <tr key={item.id}>
                    <td>{item.name}</td>
                    <td>{item.kind}</td>
                    <td>{item.canonicalUri}</td>
                    <td><span className={`status-pill${item.enabled ? " is-on" : " is-off"}`}>{item.enabled ? t("enabled") : t("disabled")}</span></td>
                    <td>
                      <div className="table-actions">
                        <Button className="table-action-button" size="sm" type="button" variant="outline" onClick={() => openEditDialog(item)}>
                          <Pencil size={14} />
                          <span>{t("edit")}</span>
                        </Button>
                        <Button className="table-action-button" size="sm" type="button" variant="outline" onClick={() => void toggleStorageRoot(item)}>
                          <Power size={14} />
                          <span>{item.enabled ? t("deactivate") : t("activate")}</span>
                        </Button>
                        <Button className="table-action-button is-danger" size="sm" type="button" variant="outline" onClick={() => void deleteStorageRoot(item)}>
                          <Trash2 size={14} />
                          <span>{t("delete")}</span>
                        </Button>
                      </div>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </Panel>
    </div>
  );
}

function StatisticsPage({ t, assetTotal, users, libraries }: TranslatorContext & { assetTotal: number; users: TeamUser[]; libraries: TeamLibrary[] }) {
  return (
    <div className="page-grid">
      <Panel title={t("assetTypes")} icon={BarChart3} className="span-6" action={<Badge>{t("placeholderData")}</Badge>}>
        <BarList t={t} items={assetBreakdown} />
      </Panel>
      <Panel title={t("imports")} icon={Network} className="span-6" action={<Badge>{t("placeholderData")}</Badge>}>
        <TrendBars />
      </Panel>
      <Panel title={t("statistics")} icon={RefreshCw} className="span-6" action={<Badge>{t("realData")}</Badge>}>
        <InfoStack items={[[t("totalAssets"), String(assetTotal)], [t("users"), String(users.length)], [t("libraries"), String(libraries.length)]]} />
      </Panel>
      <Panel title={t("auditEvents")} icon={ListChecks} className="span-6" action={<Badge>{t("placeholderData")}</Badge>}>
        <InfoStack items={[[t("activity"), t("plannedNote")], [t("retention"), "365 days"], [t("status"), t("healthy")]]} />
      </Panel>
    </div>
  );
}

function ActivityPage({ t, activityItems }: TranslatorContext & { activityItems: ActivityItem[] }) {
  return (
    <div className="page-grid">
      <Panel title={t("activity")} icon={ListChecks} className="span-12" action={<Badge>{activityItems.length ? t("realData") : t("placeholderData")}</Badge>}>
        <ActivityList t={t} activityItems={activityItems} />
      </Panel>
    </div>
  );
}

function BackupsPage({ t, deploymentMode }: TranslatorContext & { deploymentMode: DeploymentMode }) {
  return (
    <div className="page-grid">
      <Panel title={t("backups")} icon={Archive} className="span-7" action={<button className="primary-button is-disabled" type="button">{t("backupNow")}</button>}>
        <InfoStack
          items={[
            [t("lastBackup"), t("plannedNote")],
            [t("backupTarget"), deploymentMode === "local" ? "E:\\MadLibraryBackups" : "s3://madlibrary-backups"],
            [t("status"), t("prototype")]
          ]}
        />
      </Panel>
      <Panel title={t("restore")} icon={RefreshCw} className="span-5">
        <div className="placeholder-box">
          <Lock size={18} />
          <span>{t("plannedNote")}</span>
        </div>
      </Panel>
    </div>
  );
}

function SettingsPage({ t, deploymentMode, serverInfo, currentUser }: TranslatorContext & { deploymentMode: DeploymentMode; serverInfo: ServerInfo | null; currentUser: CurrentUser | null }) {
  return (
    <div className="page-grid">
      <Panel title={t("settings")} icon={Settings} className="span-6">
        <InfoStack
          items={[
            [t("deployment"), deploymentModeLabel(t, deploymentMode)],
            [t("license"), "Team Prototype"],
            [t("updateChannel"), "Beta"],
            [t("logs"), "info"]
          ]}
        />
      </Panel>
      <Panel title={t("serverInfo")} icon={Server} className="span-6">
        <InfoStack
          items={[
            [t("signedInAs"), currentUser ? `${currentUser.displayName} (${roleLabel(t, currentUser.role)})` : "-"],
            [t("serverUrl"), serverInfo?.serverUrl ?? "-"],
            [t("serverStorageDir"), serverInfo?.storageDir ?? "-"],
            [t("storage"), storageStatusLabel(t, serverInfo)],
            [t("adminAssets"), serverInfo?.adminAvailable ? t("configured") : t("notConfigured")]
          ]}
        />
      </Panel>
    </div>
  );
}

function SetupOwnerForm({ t, onDone }: TranslatorContext & { onDone: (response: { accessToken: string; user: CurrentUser }) => void }) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [error, setError] = useState("");

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    try {
      const response = await api.createOwner({ email, password, displayName: displayName || undefined });
      onDone(response);
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : String(nextError));
    }
  };

  return (
    <form className="auth-form" onSubmit={submit}>
      <p>{t("setupHint")}</p>
      <TextField label={t("email")} value={email} onChange={setEmail} />
      <TextField label={t("password")} value={password} onChange={setPassword} type="password" />
      <TextField label={t("displayName")} value={displayName} onChange={setDisplayName} />
      {error && <div className="form-error">{error}</div>}
      <button className="primary-button" type="submit">{t("createOwner")}</button>
    </form>
  );
}

function LoginForm({ t, onDone }: TranslatorContext & { onDone: (response: { accessToken: string; user: CurrentUser }) => void }) {
  const [email, setEmail] = useState(() => localStorage.getItem(rememberedLoginEmailStorageKey) ?? "");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    try {
      const normalizedEmail = email.trim();
      const response = await api.login({ email: normalizedEmail, password });
      localStorage.setItem(rememberedLoginEmailStorageKey, normalizedEmail);
      onDone(response);
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : String(nextError));
    }
  };

  return (
    <form className="auth-form" onSubmit={submit}>
      <p>{t("loginHint")}</p>
      <TextField label={t("email")} value={email} onChange={setEmail} />
      <TextField label={t("password")} value={password} onChange={setPassword} type="password" />
      {error && <div className="form-error">{error}</div>}
      <button className="primary-button" type="submit">{t("login")}</button>
    </form>
  );
}

function AuthShell({
  t,
  language,
  setLanguage,
  colorTheme,
  title,
  children
}: TranslatorContext & {
  language: Language;
  setLanguage: (language: Language) => void;
  colorTheme: ColorTheme;
  title: string;
  children?: React.ReactNode;
}) {
  return (
    <div className={`auth-shell ${colorTheme === "dark" ? "dark theme-dark" : "theme-light"}`}>
      <div className="auth-frame">
        <section className="auth-visual">
          <div className="auth-product">
            <div className="auth-logo-lockup">
              <div className="brand-mark"><img alt="" src={logoImage} /></div>
              <div>
                <div className="auth-product-name">{t("appName")}</div>
                <div className="auth-product-mode">{t("managedRuntime")}</div>
              </div>
            </div>
            <StatusDot label={t("online")} tone="good" />
          </div>

          <div className="auth-visual-copy">
            <div className="auth-kicker">{t("commandCenter")}</div>
            <h2>{t("controlPlane")}</h2>
            <p>{t("secureInit")}</p>
          </div>

          <div className="auth-system-grid">
            <div className="auth-system-card">
              <Server size={18} />
              <span>{t("localNode")}</span>
              <strong>127.0.0.1</strong>
            </div>
            <div className="auth-system-card">
              <Database size={18} />
              <span>{t("databaseOnline")}</span>
              <strong>PostgreSQL</strong>
            </div>
            <div className="auth-system-card">
              <Network size={18} />
              <span>{t("apiGateway")}</span>
              <strong>/api/v1</strong>
            </div>
            <div className="auth-system-card">
              <HardDrive size={18} />
              <span>{t("storageReady")}</span>
              <strong>{t("local")}</strong>
            </div>
          </div>

          <div className="auth-boot-panel">
            <div className="auth-boot-title">{t("bootSequence")}</div>
            <div className="auth-boot-row"><Check size={14} /> {t("databaseOnline")}</div>
            <div className="auth-boot-row"><Check size={14} /> {t("storageReady")}</div>
            <div className="auth-boot-row is-pending"><ShieldCheck size={14} /> {t("ownerProfile")}</div>
          </div>
        </section>

        <section className="auth-card">
          <div className="brand auth-brand">
            <div className="auth-brand-lockup">
              <div className="brand-mark"><img alt="" src={logoImage} /></div>
              <div>
                <div className="brand-title">{t("appName")}</div>
                <div className="brand-subtitle">{t("admin")}</div>
              </div>
            </div>
            <button className="select-button auth-language-button" type="button" onClick={() => setLanguage(language === "zh" ? "en" : "zh")}>
              <Languages size={16} />
              <span>{language === "zh" ? "\u4e2d\u6587" : "English"}</span>
            </button>
          </div>
          <div className="auth-header">
            <h1>{title}</h1>
          </div>
          {children ?? <div className="auth-note">{t("loading")}</div>}
        </section>
      </div>
    </div>
  );
}

type TranslatorContext = {
  t: ReturnType<typeof createTranslator>;
};

type PageContext = TranslatorContext & {
  language: Language;
  setLanguage: (language: Language) => void;
  deploymentMode: DeploymentMode;
  setDeploymentMode: (mode: DeploymentMode) => void;
  serviceRunning: boolean;
  serverInfo: ServerInfo | null;
  currentUser: CurrentUser | null;
  libraries: TeamLibrary[];
  users: TeamUser[];
  storageRoots: StorageRoot[];
  libraryMembers: LibraryMember[];
  selectedLibrary: TeamLibrary | null;
  selectedLibraryId: string;
  setSelectedLibraryId: (id: string) => void;
  token: string;
  assetTotal: number;
  activityItems: ActivityItem[];
  refreshAll: () => Promise<void>;
  setMessage: (message: string | null) => void;
  previewMode: boolean;
};

function Panel({
  title,
  icon: Icon,
  className = "",
  action,
  children
}: {
  title: string;
  icon: typeof Server;
  className?: string;
  action?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <section className={`panel ${className}`}>
      <div className="panel-header">
        <div className="panel-title">
          <Icon size={18} />
          <span>{title}</span>
        </div>
        {action}
      </div>
      <div className="panel-body">{children}</div>
    </section>
  );
}

function MetricCard({ label, value, change, tone }: { label: string; value: string; change: string; tone: string }) {
  return (
    <div className={`metric-card tone-${tone}`}>
      <div className="metric-label">{label}</div>
      <div className="metric-row">
        <span className="metric-value">{value}</span>
        <span className="metric-change">{change}</span>
      </div>
    </div>
  );
}

function StatusDot({ label, tone }: { label: string; tone: "good" | "warn" | "muted" }) {
  return (
    <div className={`status-dot tone-${tone}`}>
      <span />
      {label}
    </div>
  );
}

function HealthRow({ icon: Icon, label, value, tone }: { icon: typeof Server; label: string; value: string; tone: "good" | "warn" | "muted" }) {
  return (
    <div className="health-row">
      <div className="health-icon"><Icon size={17} /></div>
      <div>
        <div className="health-label">{label}</div>
        <StatusDot label={value} tone={tone} />
      </div>
    </div>
  );
}

function DataTable({ columns, rows, emptyLabel }: { columns: string[]; rows: string[][]; emptyLabel: string }) {
  return (
    <div className="table-wrap">
      <table className="data-table">
        <thead>
          <tr>
            {columns.map((column) => (
              <th key={column}>{column}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.length === 0 ? (
            <tr>
              <td colSpan={columns.length}>{emptyLabel}</td>
            </tr>
          ) : (
            rows.map((row, index) => (
              <tr key={`${row[0]}-${index}`}>
                {row.map((cell, cellIndex) => (
                  <td key={`${cell}-${cellIndex}`}>{cell}</td>
                ))}
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}

function ActivityList({ t, activityItems, compact = false }: TranslatorContext & { activityItems: ActivityItem[]; compact?: boolean }) {
  const rows = activityItems.length
    ? activityItems.map((item) => ({
        actor: item.actorDisplayName ?? item.actorEmail ?? item.actorUserId ?? t("system"),
        action: activityActionLabel(t, item.action),
        target: item.targetName ?? item.targetType ?? t("unknownTarget"),
        time: new Date(item.createdAt).toLocaleString()
      }))
    : mockActivity.map((item) => ({
        actor: item.actor === "system" ? t("system") : item.actor,
        action: t(item.action),
        target: item.target,
        time: item.time === "yesterday" ? t("yesterday") : item.time
      }));

  return (
    <div className={`activity-list${compact ? " is-compact" : ""}`}>
      {rows.map((item) => (
        <div className="activity-item" key={`${item.actor}-${item.time}-${item.action}`}>
          <div className="activity-avatar">{item.actor.slice(0, 1).toUpperCase()}</div>
          <div className="activity-main">
            <div className="activity-action">{item.action}</div>
            <div className="activity-meta">
              {item.actor} / {item.target}
            </div>
          </div>
          <div className="activity-time">{item.time}</div>
        </div>
      ))}
      {!compact && (
        <div className="activity-footer">
          <StatusDot label={activityItems.length ? t("realData") : t("placeholderData")} tone={activityItems.length ? "good" : "muted"} />
        </div>
      )}
    </div>
  );
}

function InfoStack({ items }: { items: Array<[string, string]> }) {
  return (
    <div className="info-stack">
      {items.map(([label, value]) => (
        <KeyValue key={label} label={label} value={value} />
      ))}
    </div>
  );
}

function KeyValue({ label, value }: { label: string; value: string }) {
  return (
    <div className="key-value">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function Segmented({
  value,
  options,
  onChange
}: {
  value: string;
  options: Array<{ value: string; label: string; icon: typeof Server }>;
  onChange: (value: string) => void;
}) {
  return (
    <div className="segmented">
      {options.map((option) => {
        const Icon = option.icon;
        return (
          <button
            className={option.value === value ? "is-active" : ""}
            key={option.value}
            type="button"
            onClick={() => onChange(option.value)}
          >
            <Icon size={15} />
            <span>{option.label}</span>
          </button>
        );
      })}
    </div>
  );
}

function CheckCell({ checked }: { checked: boolean }) {
  return <span className={`check-cell${checked ? " is-checked" : ""}`}>{checked ? <Check size={15} /> : "-"}</span>;
}

function BarList({ t, items }: TranslatorContext & { items: ReadonlyArray<{ label: TranslationKey; value: number }> }) {
  return (
    <div className="bar-list">
      {items.map((item) => (
        <div className="bar-item" key={item.label}>
          <div className="bar-meta">
            <span>{t(item.label)}</span>
            <strong>{item.value}%</strong>
          </div>
          <div className="bar-track">
            <span style={{ width: `${item.value}%` }} />
          </div>
        </div>
      ))}
    </div>
  );
}

function TrendBars() {
  const values = [24, 36, 28, 54, 48, 62, 74, 58, 69, 82, 77, 88];
  return (
    <div className="trend-bars">
      {values.map((value, index) => (
        <span key={`${value}-${index}`} style={{ height: `${value}%` }} />
      ))}
    </div>
  );
}

function TextField({
  label,
  value,
  onChange,
  type = "text",
  placeholder,
  required = false,
  autoFocus = false
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  type?: string;
  placeholder?: string;
  required?: boolean;
  autoFocus?: boolean;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      <input
        autoFocus={autoFocus}
        required={required}
        type={type}
        value={value}
        placeholder={placeholder}
        onChange={(event) => onChange(event.target.value)}
      />
    </label>
  );
}

function Badge({ children }: { children: React.ReactNode }) {
  return <span className="badge">{children}</span>;
}

function formatCount(value?: number | null) {
  const count = value ?? 0;
  return count === 0 ? "-" : new Intl.NumberFormat().format(count);
}

function formatBytes(value?: number | null) {
  const bytes = value ?? 0;
  if (bytes <= 0) return "-";
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  const unitIndex = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const amount = bytes / 1024 ** unitIndex;
  return `${amount >= 10 || unitIndex === 0 ? amount.toFixed(0) : amount.toFixed(1)} ${units[unitIndex]}`;
}

function parseStorageSize(value: string) {
  const match = value.trim().match(/^([\d.]+)\s*([KMGTPE]?B)$/i);
  if (!match) return 0;
  const amount = Number(match[1]);
  const unit = match[2].toUpperCase();
  const units = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];
  const unitIndex = units.indexOf(unit);
  return Number.isFinite(amount) && unitIndex >= 0 ? Math.round(amount * 1024 ** unitIndex) : 0;
}

function formatMemberNames(memberNames: string[] | undefined, fallback: string) {
  if (!memberNames || memberNames.length === 0) return fallback;
  if (memberNames.length <= 2) return memberNames.join(", ");
  return `${memberNames.slice(0, 2).join(", ")} +${memberNames.length - 2}`;
}

function roleLabel(t: ReturnType<typeof createTranslator>, role: string) {
  if (role === "owner") return t("owner");
  if (role === "admin" || role === "adminRole") return t("adminRole");
  if (role === "library_manager" || role === "manager") return t("manager");
  if (role === "editor") return t("editor");
  if (role === "viewer") return t("viewer");
  return role;
}

function activityActionLabel(t: ReturnType<typeof createTranslator>, action: string) {
  switch (action) {
    case "library.created":
      return t("libraryCreated");
    case "library.updated":
      return t("libraryUpdated");
    case "library.deleted":
      return t("libraryDeleted");
    case "library.member_upserted":
      return t("memberUpdated");
    case "library.member_removed":
      return t("memberRemoved");
    case "user.created":
      return t("userCreated");
    case "user.updated":
      return t("userUpdated");
    case "storage_root.created":
      return t("storageRootCreated");
    case "storage_root.updated":
      return t("storageRootUpdated");
    case "storage_root.deleted":
      return t("storageRootDeleted");
    default:
      return action;
  }
}

function deploymentModeLabel(t: ReturnType<typeof createTranslator>, mode: DeploymentMode) {
  return mode === "local" ? t("local") : t("cloud");
}

function storageStatusLabel(t: ReturnType<typeof createTranslator>, serverInfo: ServerInfo | null) {
  if (!serverInfo) return "-";
  if (serverInfo.storageStatus === "writable") return t("storageWritable");
  if (serverInfo.storageStatus === "read_only") return t("storageReadOnly");
  return t("storageMissing");
}

function splitList(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}
