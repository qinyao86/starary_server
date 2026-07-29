import { getCurrentWindow } from "@tauri-apps/api/window";
import { ArrowLeft, ArrowRight } from "lucide-react";
import { useCallback, useEffect, useState, type MouseEvent } from "react";
import logoImage from "../assets/logo-titlebar.png";
import type { TranslatorContext } from "../types";

function isTauriRuntime() {
  const internals = (window as typeof window & {
    __TAURI_INTERNALS__?: { invoke?: unknown };
  }).__TAURI_INTERNALS__;
  return typeof internals?.invoke === "function";
}

function SidebarLayoutIcon({ collapsed }: { collapsed: boolean }) {
  return (
    <svg aria-hidden="true" className="desktop-layout-icon" fill="none" height="16" viewBox="0 0 20 15" width="20">
      <rect x="2.5" y="2.25" width="15" height="10.5" rx="2.5" />
      <path d={collapsed ? "M6 5v5" : "M6.8 2.25v10.5"} />
    </svg>
  );
}

function MinimizeIcon() {
  return <svg aria-hidden="true" className="desktop-window-control-icon" viewBox="0 0 14 14"><path d="M2.5 7.5h9" /></svg>;
}

function MaximizeIcon({ maximized }: { maximized: boolean }) {
  return maximized
    ? <svg aria-hidden="true" className="desktop-window-control-icon" viewBox="0 0 14 14"><path d="M5.5 2.5h6v6" /><rect x="2.5" y="5.5" width="7" height="6" /></svg>
    : <svg aria-hidden="true" className="desktop-window-control-icon" viewBox="0 0 14 14"><rect x="2.5" y="2.5" width="9" height="9" /></svg>;
}

function CloseIcon() {
  return <svg aria-hidden="true" className="desktop-window-control-icon" viewBox="0 0 14 14"><path d="M2.5 2.5l9 9M11.5 2.5l-9 9" /></svg>;
}

export function DesktopTitlebar({
  canNavigateBack,
  canNavigateForward,
  collapsed,
  navigationVisible,
  t,
  onNavigateBack,
  onNavigateForward,
  onToggleSidebar
}: TranslatorContext & {
  canNavigateBack: boolean;
  canNavigateForward: boolean;
  collapsed: boolean;
  navigationVisible: boolean;
  onNavigateBack: () => void;
  onNavigateForward: () => void;
  onToggleSidebar: () => void;
}) {
  const [desktop, setDesktop] = useState(false);
  const [maximized, setMaximized] = useState(false);

  const syncMaximized = useCallback(() => {
    if (isTauriRuntime()) {
      void getCurrentWindow().isMaximized().then(setMaximized).catch(() => {});
    }
  }, []);

  useEffect(() => {
    const ready = isTauriRuntime();
    setDesktop(ready);
    if (!ready) return;
    syncMaximized();
    const unlisten = getCurrentWindow().onResized(syncMaximized);
    return () => { void unlisten.then((dispose) => dispose()); };
  }, [syncMaximized]);

  if (!desktop) return null;

  const startDrag = (event: MouseEvent<HTMLDivElement>) => {
    if (event.button === 0) {
      void getCurrentWindow().startDragging();
    }
  };

  const toggleMaximize = () => {
    void getCurrentWindow().toggleMaximize().then(syncMaximized);
  };

  const handleDragRegionDoubleClick = (event: MouseEvent<HTMLDivElement>) => {
    if (event.button === 0) toggleMaximize();
  };

  return (
    <header className="desktop-titlebar">
      <div className="desktop-titlebar-navigation" aria-label={t("pageNavigation")}>
        {navigationVisible ? (
          <>
            <button aria-label={collapsed ? t("expandSidebar") : t("collapseSidebar")} onClick={onToggleSidebar} title={collapsed ? t("expandSidebar") : t("collapseSidebar")} type="button">
              <SidebarLayoutIcon collapsed={collapsed} />
            </button>
            <button aria-label={t("navigateBack")} disabled={!canNavigateBack} onClick={onNavigateBack} title={t("navigateBack")} type="button">
              <ArrowLeft size={15} />
            </button>
            <button aria-label={t("navigateForward")} disabled={!canNavigateForward} onClick={onNavigateForward} title={t("navigateForward")} type="button">
              <ArrowRight size={15} />
            </button>
          </>
        ) : (
          <div className="desktop-titlebar-brand">
            <img alt="" src={logoImage} />
            <span>{t("appName")}</span>
          </div>
        )}
      </div>
      <div
        className="desktop-titlebar-drag-region"
        onDoubleClick={handleDragRegionDoubleClick}
        onMouseDown={startDrag}
      />
      <div className="desktop-window-actions">
        <button aria-label={t("minimizeWindow")} onClick={() => void getCurrentWindow().minimize()} title={t("minimizeWindow")} type="button"><MinimizeIcon /></button>
        <button aria-label={maximized ? t("restoreWindow") : t("maximizeWindow")} onClick={toggleMaximize} title={maximized ? t("restoreWindow") : t("maximizeWindow")} type="button"><MaximizeIcon maximized={maximized} /></button>
        <button className="desktop-window-close" aria-label={t("closeWindow")} onClick={() => void getCurrentWindow().close()} title={t("closeWindow")} type="button"><CloseIcon /></button>
      </div>
    </header>
  );
}
