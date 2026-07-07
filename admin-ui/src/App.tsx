import { useMemo, useState } from "react";
import { AdminShell } from "./components/admin-shell";
import { AuthShell, LoginForm, SetupOwnerForm } from "./components/auth";
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
  const title = t(navItems.find((item) => item.id === section)?.label ?? "overview");

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
    libraryActivityItems,
    refreshAll,
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
      onToggleColorTheme={toggleColorTheme}
    >
      {renderSection(section, context)}
    </AdminShell>
  );
}
