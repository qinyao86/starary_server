import { useMemo, useState } from "react";
import { AdminShell } from "./components/admin-shell";
import { AuthShell, FirstRunSetup, LoginForm } from "./components/auth";
import { colorThemeStorageKey, sectionStorageKey } from "./constants";
import { useAdminRuntime } from "./hooks/use-admin-runtime";
import { createTranslator, type Language } from "./i18n";
import { navItems, readStoredSection } from "./navigation";
import { renderSection } from "./pages/render-section";
import type { ColorTheme, PageContext, Section } from "./types";
import { buildPreviewLibraries, buildPreviewUsers, previewCurrentUser } from "./utils/preview";

export function App() {
  const [language, setLanguage] = useState<Language>("zh");
  const [section, setSection] = useState<Section>(() => readStoredSection());
  const [showFirstLibrarySetup, setShowFirstLibrarySetup] = useState(false);
  const runtime = useAdminRuntime();
  const [colorTheme, setColorTheme] = useState<ColorTheme>(() => {
    const stored = localStorage.getItem(colorThemeStorageKey);
    return stored === "light" ? "light" : "dark";
  });
  const t = useMemo(() => createTranslator(language), [language]);
  const {
    activityItems,
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
    onAuthenticated,
    previewMode,
    refreshAll,
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

  const selectSection = (nextSection: Section) => {
    localStorage.setItem(sectionStorageKey, nextSection);
    setSection(nextSection);
  };

  const finishFirstLibrarySetup = async (libraryId?: string) => {
    if (libraryId) {
      selectLibrary(libraryId);
      await refreshAll();
    }
    selectSection("libraries");
    setShowFirstLibrarySetup(false);
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

  if ((needsOwner || showFirstLibrarySetup) && !previewMode) {
    return (
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
    navigateToSection: selectSection,
    setMessage,
    previewMode
  };

  return (
    <AdminShell
      colorTheme={colorTheme}
      deploymentMode={deploymentMode}
      message={message}
      previewMode={previewMode}
      section={section}
      serverInfo={serverInfo}
      serviceRunning={serviceRunning}
      t={t}
      title={title}
      onClearMessage={() => setMessage(null)}
      onLogout={logout}
      onRefresh={refreshAll}
      onSelectSection={selectSection}
      onSetDeploymentMode={setDeploymentMode}
    >
      {renderSection(section, context)}
    </AdminShell>
  );
}
