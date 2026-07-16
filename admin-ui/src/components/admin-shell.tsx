import { openUrl } from "@tauri-apps/plugin-opener";
import { Cloud, LogOut, MoreHorizontal, RefreshCw, Server } from "lucide-react";
import { useEffect, useRef, useState, type ReactNode } from "react";
import logoImage from "../assets/logo.png";
import { api, type CurrentUser, type ServerInfo, type SystemAvatar } from "../api";
import { visibleNavItems } from "../navigation";
import type { ColorTheme, DeploymentMode, Section, TranslatorContext } from "../types";
import { defaultSystemAvatars } from "../utils/avatars";
import { deploymentModeLabel, roleLabel } from "../utils/format";
import { Segmented, UserAvatar } from "./common";
import { AvatarDialog } from "./dialogs";

export function AdminShell({
  children,
  colorTheme,
  currentUser,
  deploymentMode,
  message,
  previewMode,
  currentUserRole,
  section,
  sidebarCollapsed,
  serverInfo,
  serviceRunning,
  t,
  title,
  token,
  onClearMessage,
  onCreateBrowserHandoff,
  onLogout,
  onRefresh,
  onSelectSection,
  onSetMessage,
  onSetDeploymentMode
}: TranslatorContext & {
  children: ReactNode;
  colorTheme: ColorTheme;
  currentUser: CurrentUser | null;
  currentUserRole: string | null;
  deploymentMode: DeploymentMode;
  message: string | null;
  previewMode: boolean;
  section: Section;
  sidebarCollapsed: boolean;
  serverInfo: ServerInfo | null;
  serviceRunning: boolean;
  title: string;
  token: string;
  onClearMessage: () => void;
  onCreateBrowserHandoff: () => Promise<string | null>;
  onLogout: () => void;
  onRefresh: () => Promise<void>;
  onSelectSection: (section: Section) => void;
  onSetMessage: (message: string | null) => void;
  onSetDeploymentMode: (mode: DeploymentMode) => void;
}) {
  useEffect(() => {
    if (!message) return;
    const timer = window.setTimeout(onClearMessage, 2200);
    return () => window.clearTimeout(timer);
  }, [message, onClearMessage]);

  return (
    <div className={`admin-shell${sidebarCollapsed ? " is-sidebar-collapsed" : ""} ${colorTheme === "dark" ? "dark theme-dark" : "theme-light"}`}>
      <aside className="sidebar">
        <SidebarBrand t={t} />
        <SidebarStatus serverInfo={serverInfo} serviceRunning={serviceRunning} t={t} onCreateBrowserHandoff={onCreateBrowserHandoff} />
        <SidebarNav currentUserRole={currentUserRole} section={section} t={t} onSelectSection={onSelectSection} />
        <SidebarFooter currentUser={currentUser} t={t} token={token} onLogout={onLogout} onRefresh={onRefresh} onSetMessage={onSetMessage} />
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
              onChange={(value) => onSetDeploymentMode(value as DeploymentMode)}
            />
            <button className="secondary-button" type="button" onClick={() => void onRefresh()}>
              <RefreshCw size={16} />
              <span>{t("refresh")}</span>
            </button>
          </div>
        </header>

        {message && <div className="toast is-visible" role="status" aria-live="polite">{message}</div>}

        <section className="content">
          <div className="content-inner">{children}</div>
        </section>
      </main>
    </div>
  );
}

function SidebarBrand({ t }: TranslatorContext) {
  return (
    <div className="brand">
      <div className="brand-mark">
        <img alt="" src={logoImage} />
      </div>
      <div>
        <div className="brand-title">{t("appName")}</div>
        <div className="brand-subtitle">{t("admin")}</div>
      </div>
    </div>
  );
}

function SidebarStatus({ serverInfo, serviceRunning, t, onCreateBrowserHandoff }: TranslatorContext & {
  serverInfo: ServerInfo | null;
  serviceRunning: boolean;
  onCreateBrowserHandoff: () => Promise<string | null>;
}) {
  const adminUrl = serverInfo?.lanAdminUrl ?? serverInfo?.localAdminUrl ?? `${serverInfo?.serverUrl ?? "http://127.0.0.1:3789"}/admin/`;
  const address = adminUrl.replace(/^https?:\/\//, "").replace(/\/admin\/?$/, "");

  const handleAddressClick = async (event: React.MouseEvent<HTMLAnchorElement>) => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    event.preventDefault();
    let browserUrl = adminUrl;
    try {
      const code = await onCreateBrowserHandoff();
      if (code) {
        const url = new URL(adminUrl);
        url.hash = new URLSearchParams({ handoff: code }).toString();
        browserUrl = url.toString();
      }
    } catch {
      // Opening the address without a handoff is still useful if the server is restarting.
    }
    void openUrl(browserUrl);
  };

  return (
    <div className="sidebar-status-area" title={`${t(serviceRunning ? "running" : "stopped")} - ${address}`}>
      <div className={`sidebar-runtime${serviceRunning ? " is-running" : " is-stopped"}`}>
        <div className="sidebar-runtime-heading">
          <span className="sidebar-runtime-indicator" aria-hidden="true" />
          <strong>{t(serviceRunning ? "running" : "stopped")}</strong>
        </div>
        <a
          className="sidebar-runtime-address"
          href={adminUrl}
          rel="noreferrer"
          title={t("openAdminInBrowser")}
          target="_blank"
          onClick={(event) => { void handleAddressClick(event); }}
        >
          <Server size={15} aria-hidden="true" />
          <code>{address}</code>
        </a>
      </div>
    </div>
  );
}

function SidebarNav({ currentUserRole, section, t, onSelectSection }: TranslatorContext & {
  currentUserRole: string | null;
  section: Section;
  onSelectSection: (section: Section) => void;
}) {
  return (
    <nav className="nav-list" aria-label="Server admin navigation">
      {visibleNavItems(currentUserRole).map((item) => {
        const Icon = item.icon;
        const active = item.id === section;
        return (
          <button
            className={`nav-item${active ? " is-active" : ""}`}
            key={item.id}
            type="button"
            title={t(item.label)}
            onClick={() => onSelectSection(item.id)}
          >
            <Icon size={17} />
            <span>{t(item.label)}</span>
          </button>
        );
      })}
    </nav>
  );
}

function SidebarFooter({ currentUser, t, token, onLogout, onRefresh, onSetMessage }: TranslatorContext & {
  currentUser: CurrentUser | null;
  token: string;
  onLogout: () => void;
  onRefresh: () => Promise<void>;
  onSetMessage: (message: string | null) => void;
}) {
  const [menuOpen, setMenuOpen] = useState(false);
  const [avatarOpen, setAvatarOpen] = useState(false);
  const [avatarBusy, setAvatarBusy] = useState(false);
  const [avatars, setAvatars] = useState<SystemAvatar[]>(defaultSystemAvatars);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const displayName = currentUser?.displayName?.trim() || currentUser?.email || t("standardUser");
  const email = currentUser?.email ?? "";
  const title = email ? `${displayName} - ${email}` : displayName;

  useEffect(() => {
    if (!menuOpen) return;
    const handlePointerDown = (event: PointerEvent) => {
      if (menuRef.current?.contains(event.target as Node)) return;
      setMenuOpen(false);
    };
    window.addEventListener("pointerdown", handlePointerDown);
    return () => window.removeEventListener("pointerdown", handlePointerDown);
  }, [menuOpen]);

  const openAvatarDialog = async () => {
    if (!currentUser || !token) return;
    setAvatarOpen(true);
    try {
      setAvatars(await api.listSystemAvatars(token));
    } catch {
      setAvatars(defaultSystemAvatars);
    }
  };

  const updateAvatar = async (avatarKey: string) => {
    if (!currentUser || !token) return;
    setAvatarBusy(true);
    try {
      await api.updateUserAvatar(token, currentUser.id, avatarKey);
      setAvatarOpen(false);
      onSetMessage(t("avatarUpdated"));
      await onRefresh();
    } catch (error) {
      onSetMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setAvatarBusy(false);
    }
  };

  return (
    <div className="sidebar-footer" ref={menuRef}>
      <AvatarDialog
        avatars={avatars}
        busy={avatarBusy}
        currentAvatarKey={currentUser?.avatarKey}
        open={avatarOpen}
        t={t}
        targetName={displayName}
        onClose={() => !avatarBusy && setAvatarOpen(false)}
        onSelect={(avatarKey) => { void updateAvatar(avatarKey); }}
      />
      <div className="sidebar-account" title={title}>
        <UserAvatar avatarKey={currentUser?.avatarKey} label={displayName} size="lg" onClick={() => { void openAvatarDialog(); }} />
        <div className="sidebar-account-main">
          <div className="sidebar-account-name">{displayName}</div>
          {currentUser?.role && <div className="sidebar-account-role">{roleLabel(t, currentUser.role)}</div>}
        </div>
        <button
          aria-expanded={menuOpen}
          aria-label={t("accountIdentity")}
          className="sidebar-account-menu-button"
          type="button"
          title={t("accountIdentity")}
          onClick={() => setMenuOpen((open) => !open)}
        >
          <MoreHorizontal size={16} />
        </button>
      </div>
      {menuOpen && (
        <div className="sidebar-account-menu" role="menu">
          <div className="sidebar-account-menu-header">
            <strong>{displayName}</strong>
            {email && <span>{email}</span>}
            {currentUser?.role && <span>{roleLabel(t, currentUser.role)}</span>}
          </div>
          <button className="sidebar-logout-button" type="button" role="menuitem" onClick={onLogout}>
            <LogOut size={15} />
            <span>{t("logout")}</span>
          </button>
        </div>
      )}
    </div>
  );
}
