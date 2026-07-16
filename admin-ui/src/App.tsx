import { useEffect, useMemo, useState, type ReactNode } from "react";
import { api } from "./api";
import { AdminShell } from "./components/admin-shell";
import { AuthShell, FirstRunSetup, LoginForm } from "./components/auth";
import { DesktopTitlebar } from "./components/desktop-titlebar";
import { colorThemeStorageKey, historyMaxIndexStorageKey, languageStorageKey, sectionStorageKey, sidebarCollapsedStorageKey } from "./constants";
import { useAdminRuntime } from "./hooks/use-admin-runtime";
import { createTranslator, type Language } from "./i18n";
import { canAccessSection, libraryIdFromPath, libraryPath, navItems, readStoredSection, sectionFromPath, sectionPath } from "./navigation";
import { renderSection } from "./pages/render-section";
import type { ColorTheme, PageContext, Section } from "./types";
import { buildPreviewLibraries, buildPreviewUsers, previewCurrentUser } from "./utils/preview";

export function App() {
  const [language, setLanguage] = useState<Language>(() => {
    const stored = localStorage.getItem(languageStorageKey);
    return stored === "en" ? "en" : "zh";
  });
  const [section, setSection] = useState<Section>(() => readStoredSection());
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => localStorage.getItem(sidebarCollapsedStorageKey) === "true");
  const [historyIndex, setHistoryIndex] = useState(() => readHistoryIndex());
  const [historyMaxIndex, setHistoryMaxIndex] = useState(() => readHistoryMaxIndex());
  const [libraryRouteId, setLibraryRouteId] = useState<string | null>(readRequestedLibraryId);
  const [showFirstLibrarySetup, setShowFirstLibrarySetup] = useState(false);
  const runtime = useAdminRuntime();
  const [colorTheme, setColorTheme] = useState<ColorTheme>(() => {
    const stored = localStorage.getItem(colorThemeStorageKey);
    return stored === "light" || stored === "dark" || stored === "system" ? stored : "system";
  });
  const [systemDark, setSystemDark] = useState(() => window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false);
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
  const currentUserRole = effectiveCurrentUser?.role ?? null;
  const activeSection = canAccessSection(section, currentUserRole) ? section : "libraries";
  const title = t(navItems.find((item) => item.id === activeSection)?.label ?? "libraries");
  const effectiveColorTheme: Exclude<ColorTheme, "system"> = colorTheme === "system" ? (systemDark ? "dark" : "light") : colorTheme;

  useEffect(() => {
    const media = window.matchMedia?.("(prefers-color-scheme: dark)");
    if (!media) return;
    const handleChange = (event: MediaQueryListEvent) => setSystemDark(event.matches);
    setSystemDark(media.matches);
    media.addEventListener("change", handleChange);
    return () => media.removeEventListener("change", handleChange);
  }, []);

  const updateColorTheme = (nextTheme: ColorTheme) => {
    localStorage.setItem(colorThemeStorageKey, nextTheme);
    setColorTheme(nextTheme);
  };

  const updateLanguage = (nextLanguage: Language) => {
    localStorage.setItem(languageStorageKey, nextLanguage);
    setLanguage(nextLanguage);
  };

  useEffect(() => {
    const hasExistingHistoryState = typeof (window.history.state as { index?: number } | null)?.index === "number";
    const initialIndex = readHistoryIndex();
    const initialSection = sectionFromPath(window.location.pathname) ?? section;
    const requestedLibraryId = initialSection === "libraries" ? readRequestedLibraryId() : null;
    const initialPath = requestedLibraryId ? libraryPath(requestedLibraryId) : sectionPath(initialSection);
    window.history.replaceState(
      { section: initialSection, index: initialIndex, libraryId: requestedLibraryId },
      "",
      initialPath,
    );
    setLibraryRouteId(requestedLibraryId);
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
      setLibraryRouteId(nextSection === "libraries" ? libraryIdFromPath(window.location.pathname) : null);
      setHistoryIndex(nextIndex);
    };
    window.addEventListener("popstate", handlePopState);
    return () => window.removeEventListener("popstate", handlePopState);
  }, []);

  const navigateToLibrary = (nextLibraryId: string | null, options: { replace?: boolean } = {}) => {
    const nextPath = libraryPath(nextLibraryId);
    localStorage.setItem(sectionStorageKey, "libraries");

    if (options.replace) {
      window.history.replaceState(
        { section: "libraries", index: historyIndex, libraryId: nextLibraryId },
        "",
        nextPath,
      );
      setSection("libraries");
      setLibraryRouteId(nextLibraryId);
      if (nextLibraryId) selectLibrary(nextLibraryId);
      return;
    }

    if (section === "libraries" && libraryRouteId === nextLibraryId) {
      if (nextLibraryId) selectLibrary(nextLibraryId);
      else void refreshAll();
      return;
    }

    const nextIndex = historyIndex + 1;
    window.history.pushState(
      { section: "libraries", index: nextIndex, libraryId: nextLibraryId },
      "",
      nextPath,
    );
    sessionStorage.setItem(historyMaxIndexStorageKey, String(nextIndex));
    setHistoryIndex(nextIndex);
    setHistoryMaxIndex(nextIndex);
    setSection("libraries");
    setLibraryRouteId(nextLibraryId);
    if (nextLibraryId) selectLibrary(nextLibraryId);
  };

  const selectSection = (nextSection: Section) => {
    const targetSection = canAccessSection(nextSection, currentUserRole) ? nextSection : "libraries";
    localStorage.setItem(sectionStorageKey, targetSection);

    if (targetSection === "libraries" && (section !== "libraries" || libraryRouteId)) {
      navigateToLibrary(null);
      return;
    }

    if (targetSection !== section) {
      const nextIndex = historyIndex + 1;
      window.history.pushState({ section: targetSection, index: nextIndex }, "", sectionPath(targetSection));
      sessionStorage.setItem(historyMaxIndexStorageKey, String(nextIndex));
      setHistoryIndex(nextIndex);
      setHistoryMaxIndex(nextIndex);
      setSection(targetSection);
      setLibraryRouteId(null);
    } else {
      void refreshAll();
    }
  };

  useEffect(() => {
    if (!signedIn || canAccessSection(section, currentUserRole)) return;
    const fallbackSection: Section = "libraries";
    localStorage.setItem(sectionStorageKey, fallbackSection);
    window.history.replaceState({ section: fallbackSection, index: historyIndex }, "", sectionPath(fallbackSection));
    setSection(fallbackSection);
    setLibraryRouteId(null);
  }, [currentUserRole, historyIndex, section, signedIn]);

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
    <div className={`desktop-app-frame ${effectiveColorTheme === "dark" ? "theme-dark" : "theme-light"}`}>
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
    return withDesktopFrame(<AuthShell t={t} language={language} setLanguage={updateLanguage} title={t("loading")} colorTheme={effectiveColorTheme} />);
  }

  if (apiState === "unavailable" && !previewMode) {
    return withDesktopFrame(
      <AuthShell t={t} language={language} setLanguage={updateLanguage} title={t("apiUnavailable")} colorTheme={effectiveColorTheme}>
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
        <AuthShell t={t} language={language} setLanguage={updateLanguage} title={t("setupOnServerTitle")} colorTheme={effectiveColorTheme}>
          <div className="auth-note">{t("setupOnServerHint")}</div>
        </AuthShell>
      );
    }
    return withDesktopFrame(
      <FirstRunSetup
        colorTheme={effectiveColorTheme}
        language={language}
        setLanguage={updateLanguage}
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
    return withDesktopFrame(<AuthShell t={t} language={language} setLanguage={updateLanguage} title={t("loading")} colorTheme={effectiveColorTheme} />);
  }

  if (!signedIn) {
    return withDesktopFrame(
      <AuthShell t={t} language={language} setLanguage={updateLanguage} title={t("login")} colorTheme={effectiveColorTheme}>
        <LoginForm t={t} onDone={onAuthenticated} />
      </AuthShell>
    );
  }

  const context: PageContext = {
    t,
    language,
    setLanguage: updateLanguage,
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
    libraryRouteId,
    navigateToLibrary,
    setMessage,
    previewMode
  };

  return withDesktopFrame(
    <AdminShell
      colorTheme={effectiveColorTheme}
      currentUser={effectiveCurrentUser}
      currentUserRole={currentUserRole}
      deploymentMode={deploymentMode}
      message={message}
      previewMode={previewMode}
      section={activeSection}
      sidebarCollapsed={sidebarCollapsed}
      serverInfo={serverInfo}
      serviceRunning={serviceRunning}
      t={t}
      title={title}
      token={token ?? ""}
      onClearMessage={() => setMessage(null)}
      onLogout={logout}
      onRefresh={refreshAll}
      onCreateBrowserHandoff={() => token ? api.createBrowserHandoff(token).then((response) => response.code) : Promise.resolve(null)}
      onSelectSection={selectSection}
      onSetMessage={setMessage}
      onSetDeploymentMode={setDeploymentMode}
    >
      {renderSection(activeSection, context)}
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

function readRequestedLibraryId() {
  return libraryIdFromPath(window.location.pathname)
    ?? new URLSearchParams(window.location.search).get("libraryId")?.trim()
    ?? null;
}
