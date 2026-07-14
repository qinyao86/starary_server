import { useEffect, useMemo, useState, type ReactNode } from "react";
import { api } from "./api";
import { AdminShell } from "./components/admin-shell";
import { AuthShell, FirstRunSetup, LoginForm } from "./components/auth";
import { DesktopTitlebar } from "./components/desktop-titlebar";
import { colorThemeStorageKey, historyMaxIndexStorageKey, sectionStorageKey, sidebarCollapsedStorageKey } from "./constants";
import { useAdminRuntime } from "./hooks/use-admin-runtime";
import { createTranslator, type Language } from "./i18n";
import { navItems, readStoredSection, sectionFromPath, sectionPath } from "./navigation";
import { renderSection } from "./pages/render-section";
import type { ColorTheme, PageContext, Section } from "./types";
import { buildPreviewLibraries, buildPreviewUsers, previewCurrentUser } from "./utils/preview";

export function App() {
  const [language, setLanguage] = useState<Language>("zh");
  const [section, setSection] = useState<Section>(() => readStoredSection());
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => localStorage.getItem(sidebarCollapsedStorageKey) === "true");
  const [historyIndex, setHistoryIndex] = useState(() => readHistoryIndex());
  const [historyMaxIndex, setHistoryMaxIndex] = useState(() => readHistoryMaxIndex());
  const [libraryListViewVersion, setLibraryListViewVersion] = useState(0);
  const [showFirstLibrarySetup, setShowFirstLibrarySetup] = useState(false);
  const runtime = useAdminRuntime();
  const [colorTheme, setColorTheme] = useState<ColorTheme>(() => {
    const stored = localStorage.getItem(colorThemeStorageKey);
    return stored === "light" ? "light" : "dark";
  });
  const t = useMemo(() => createTranslator(language), [language]);
  const {
    activityItems,
    authChecked,
    apiState,
    assetTotal,
    currentUser,
    deploymentMode,
    libraries,
    libraryActivityItems,
    libraryMembers,
    logout,
    message,
    needsOwner,
    ownerSetupAllowed,
    onAuthenticated,
    previewMode,
    refreshAll,
    resetAfterInitialization,
    selectLibrary,
    selectedLibraryId,
    serverInfo,
    serviceRunning,
    setDeploymentMode,
    setMessage,
    setPreviewMode,
    storageRoots,
    storageConnections,
    token,
    users
  } = runtime;

  const previewLibraries = useMemo(() => buildPreviewLibraries(t), [t]);
  const previewUsers = useMemo(() => buildPreviewUsers(), []);
  const effectiveLibraries = previewMode && libraries.length === 0 ? previewLibraries : libraries;
  const effectiveUsers = previewMode && users.length === 0 ? previewUsers : users;
  const effectiveAssetTotal = previewMode && assetTotal === 0 ? 128430 : assetTotal;
  const effectiveCurrentUser =
    currentUser ??
    (previewMode ? previewCurrentUser : null);
  const signedIn = previewMode || Boolean(token && currentUser);
  const selectedLibrary = effectiveLibraries.find((item) => item.id === selectedLibraryId) ?? effectiveLibraries[0] ?? null;
  const title = t(navItems.find((item) => item.id === section)?.label ?? "libraries");

  const updateColorTheme = (nextTheme: ColorTheme) => {
    localStorage.setItem(colorThemeStorageKey, nextTheme);
    setColorTheme(nextTheme);
  };

  useEffect(() => {
    const hasExistingHistoryState = typeof (window.history.state as { index?: number } | null)?.index === "number";
    const initialIndex = readHistoryIndex();
    const initialSection = sectionFromPath(window.location.pathname) ?? section;
    window.history.replaceState({ section: initialSection, index: initialIndex }, "", sectionPath(initialSection));
    setHistoryIndex(initialIndex);
    setHistoryMaxIndex(hasExistingHistoryState ? Math.max(readHistoryMaxIndex(), initialIndex) : initialIndex);
    if (!hasExistingHistoryState) {
      sessionStorage.setItem(historyMaxIndexStorageKey, String(initialIndex));
    }

    const handlePopState = (event: PopStateEvent) => {
      const state = event.state as { section?: Section; index?: number } | null;
      const nextSection = state?.section ?? sectionFromPath(window.location.pathname) ?? "libraries";
      const nextIndex = typeof state?.index === "number" ? state.index : 0;
      localStorage.setItem(sectionStorageKey, nextSection);
      setSection(nextSection);
      setHistoryIndex(nextIndex);
    };
    window.addEventListener("popstate", handlePopState);
    return () => window.removeEventListener("popstate", handlePopState);
  }, []);

  const selectSection = (nextSection: Section) => {
    localStorage.setItem(sectionStorageKey, nextSection);
    if (nextSection === "libraries") {
      setLibraryListViewVersion((value) => value + 1);
    }
    if (nextSection !== section) {
      const nextIndex = historyIndex + 1;
      window.history.pushState({ section: nextSection, index: nextIndex }, "", sectionPath(nextSection));
      sessionStorage.setItem(historyMaxIndexStorageKey, String(nextIndex));
      setHistoryIndex(nextIndex);
      setHistoryMaxIndex(nextIndex);
      setSection(nextSection);
    } else {
      void refreshAll();
    }
  };

  useEffect(() => {
    if (signedIn) {
      void refreshAll();
    }
  }, [section]);

  const toggleSidebar = () => {
    const next = !sidebarCollapsed;
    localStorage.setItem(sidebarCollapsedStorageKey, String(next));
    setSidebarCollapsed(next);
  };

  const withDesktopFrame = (content: ReactNode, navigationVisible = false) => (
    <div className={`desktop-app-frame ${colorTheme === "dark" ? "theme-dark" : "theme-light"}`}>
      <DesktopTitlebar
        canNavigateBack={historyIndex > 0}
        canNavigateForward={historyIndex < historyMaxIndex}
        collapsed={sidebarCollapsed}
        navigationVisible={navigationVisible}
        t={t}
        onNavigateBack={() => window.history.back()}
        onNavigateForward={() => window.history.forward()}
        onToggleSidebar={toggleSidebar}
      />
      <div className="desktop-app-content">{content}</div>
    </div>
  );

  const finishFirstLibrarySetup = async (libraryId?: string) => {
    if (libraryId) {
      selectLibrary(libraryId);
      await refreshAll();
    }
    selectSection("libraries");
    setShowFirstLibrarySetup(false);
  };

  if (apiState === "loading" && needsOwner === null) {
    return withDesktopFrame(<AuthShell t={t} language={language} setLanguage={setLanguage} title={t("loading")} colorTheme={colorTheme} />);
  }

  if (apiState === "unavailable" && !previewMode) {
    return withDesktopFrame(
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

  if ((needsOwner || showFirstLibrarySetup) && !previewMode) {
    if (needsOwner && !ownerSetupAllowed) {
      return withDesktopFrame(
        <AuthShell t={t} language={language} setLanguage={setLanguage} title={t("setupOnServerTitle")} colorTheme={colorTheme}>
          <div className="auth-note">{t("setupOnServerHint")}</div>
        </AuthShell>
      );
    }
    return withDesktopFrame(
      <FirstRunSetup
        colorTheme={colorTheme}
        language={language}
        setLanguage={setLanguage}
        t={t}
        token={token}
        onOwnerDone={(response) => {
          onAuthenticated(response);
          setShowFirstLibrarySetup(true);
        }}
        onLibraryDone={(libraryId) => finishFirstLibrarySetup(libraryId)}
        onSkip={() => void finishFirstLibrarySetup()}
      />
    );
  }

  if (!authChecked) {
    return withDesktopFrame(<AuthShell t={t} language={language} setLanguage={setLanguage} title={t("loading")} colorTheme={colorTheme} />);
  }

  if (!signedIn) {
    return withDesktopFrame(
      <AuthShell t={t} language={language} setLanguage={setLanguage} title={t("login")} colorTheme={colorTheme}>
        <LoginForm t={t} onDone={onAuthenticated} />
      </AuthShell>
    );
  }

  const context: PageContext = {
    t,
    language,
    setLanguage,
    colorTheme,
    setColorTheme: updateColorTheme,
    deploymentMode,
    setDeploymentMode,
    serviceRunning,
    serverInfo,
    currentUser: effectiveCurrentUser,
    libraries: effectiveLibraries,
    users: effectiveUsers,
    storageRoots,
    storageConnections,
    libraryMembers,
    selectedLibrary,
    selectedLibraryId: selectedLibrary?.id ?? selectedLibraryId,
    setSelectedLibraryId: selectLibrary,
    token: token ?? "",
    assetTotal: effectiveAssetTotal,
    activityItems,
    libraryActivityItems,
    refreshAll,
    resetAfterInitialization,
    navigateToSection: selectSection,
    libraryListViewVersion,
    setMessage,
    previewMode
  };

  return withDesktopFrame(
    <AdminShell
      colorTheme={colorTheme}
      deploymentMode={deploymentMode}
      message={message}
      previewMode={previewMode}
      section={section}
      sidebarCollapsed={sidebarCollapsed}
      serverInfo={serverInfo}
      serviceRunning={serviceRunning}
      t={t}
      title={title}
      onClearMessage={() => setMessage(null)}
      onLogout={logout}
      onRefresh={refreshAll}
      onCreateBrowserHandoff={() => token ? api.createBrowserHandoff(token).then((response) => response.code) : Promise.resolve(null)}
      onSelectSection={selectSection}
      onSetDeploymentMode={setDeploymentMode}
    >
      {renderSection(section, context)}
    </AdminShell>,
    true
  );
}

function readHistoryIndex() {
  const state = window.history.state as { index?: number } | null;
  return typeof state?.index === "number" ? state.index : 0;
}

function readHistoryMaxIndex() {
  const stored = Number(sessionStorage.getItem(historyMaxIndexStorageKey));
  return Number.isFinite(stored) && stored >= 0 ? stored : readHistoryIndex();
}
