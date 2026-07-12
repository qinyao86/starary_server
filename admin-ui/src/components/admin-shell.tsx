import { openUrl } from "@tauri-apps/plugin-opener";
import { Cloud, ExternalLink, LogOut, RefreshCw, Server } from "lucide-react";
import type { ReactNode } from "react";
import logoImage from "../assets/logo.png";
import type { ServerInfo } from "../api";
import { navItems } from "../navigation";
import type { ColorTheme, DeploymentMode, Section, TranslatorContext } from "../types";
import { deploymentModeLabel } from "../utils/format";
import { Segmented } from "./common";

export function AdminShell({
  children,
  colorTheme,
  deploymentMode,
  message,
  previewMode,
  section,
  sidebarCollapsed,
  serverInfo,
  serviceRunning,
  t,
  title,
  onClearMessage,
  onLogout,
  onRefresh,
  onSelectSection,
  onSetDeploymentMode
}: TranslatorContext & {
  children: ReactNode;
  colorTheme: ColorTheme;
  deploymentMode: DeploymentMode;
  message: string | null;
  previewMode: boolean;
  section: Section;
  sidebarCollapsed: boolean;
  serverInfo: ServerInfo | null;
  serviceRunning: boolean;
  title: string;
  onClearMessage: () => void;
  onLogout: () => void;
  onRefresh: () => Promise<void>;
  onSelectSection: (section: Section) => void;
  onSetDeploymentMode: (mode: DeploymentMode) => void;
}) {
  return (
    <div className={`admin-shell${sidebarCollapsed ? " is-sidebar-collapsed" : ""} ${colorTheme === "dark" ? "dark theme-dark" : "theme-light"}`}>
      <aside className="sidebar">
        <SidebarBrand t={t} />
        <SidebarStatus deploymentMode={deploymentMode} serverInfo={serverInfo} serviceRunning={serviceRunning} t={t} />
        <SidebarNav section={section} t={t} onSelectSection={onSelectSection} />
        <SidebarFooter t={t} onLogout={onLogout} />
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

        {message && (
          <div className="message-bar" role="status" aria-live="polite">
            <span>{message}</span>
            <button type="button" onClick={onClearMessage}>
              x
            </button>
          </div>
        )}

        <section className="content">{children}</section>
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

function SidebarStatus({ deploymentMode, serverInfo, serviceRunning, t }: TranslatorContext & {
  deploymentMode: DeploymentMode;
  serverInfo: ServerInfo | null;
  serviceRunning: boolean;
}) {
  const adminUrl = serverInfo?.lanAdminUrl ?? serverInfo?.localAdminUrl ?? `${serverInfo?.serverUrl ?? "http://127.0.0.1:3789"}/admin/`;
  const address = adminUrl.replace(/^https?:\/\//, "").replace(/\/admin\/?$/, "");

  const openAdmin = () => {
    if ("__TAURI_INTERNALS__" in window) {
      void openUrl(adminUrl);
    } else {
      window.open(adminUrl, "_blank", "noopener,noreferrer");
    }
  };

  return (
    <div className="sidebar-status-area" title={`${t(serviceRunning ? "running" : "stopped")} - ${address}`}>
      <div className={`sidebar-runtime${serviceRunning ? " is-running" : " is-stopped"}`}>
        <div className="sidebar-runtime-heading">
          <span className="sidebar-runtime-indicator" aria-hidden="true" />
          <strong>{t(serviceRunning ? "running" : "stopped")}</strong>
          <span>{deploymentModeLabel(t, deploymentMode)}</span>
        </div>
        <button
          aria-label={t("openAdminInBrowser")}
          className="sidebar-runtime-address"
          title={t("openAdminInBrowser")}
          type="button"
          onClick={openAdmin}
        >
          <Server size={12} aria-hidden="true" />
          <code>{address}</code>
          <ExternalLink size={11} aria-hidden="true" />
        </button>
      </div>
    </div>
  );
}

function SidebarNav({ section, t, onSelectSection }: TranslatorContext & {
  section: Section;
  onSelectSection: (section: Section) => void;
}) {
  return (
    <nav className="nav-list" aria-label="Server admin navigation">
      {navItems.map((item) => {
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

function SidebarFooter({ t, onLogout }: TranslatorContext & {
  onLogout: () => void;
}) {
  return (
    <div className="sidebar-footer">
      <button className="sidebar-logout-button" type="button" onClick={onLogout} title={t("logout")}>
        <LogOut size={15} />
        <span>{t("logout")}</span>
      </button>
    </div>
  );
}
