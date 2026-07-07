import { Cloud, LogOut, Moon, RefreshCw, Server, Sun } from "lucide-react";
import type { ReactNode } from "react";
import logoImage from "../assets/logo.png";
import type { ServerInfo } from "../api";
import { navItems } from "../navigation";
import type { ColorTheme, DeploymentMode, Section, TranslatorContext } from "../types";
import { deploymentModeLabel } from "../utils/format";
import { Segmented, StatusDot } from "./common";

export function AdminShell({
  children,
  colorTheme,
  deploymentMode,
  message,
  previewMode,
  section,
  serverInfo,
  serviceRunning,
  t,
  title,
  onClearMessage,
  onLogout,
  onRefresh,
  onSelectSection,
  onSetDeploymentMode,
  onToggleColorTheme
}: TranslatorContext & {
  children: ReactNode;
  colorTheme: ColorTheme;
  deploymentMode: DeploymentMode;
  message: string | null;
  previewMode: boolean;
  section: Section;
  serverInfo: ServerInfo | null;
  serviceRunning: boolean;
  title: string;
  onClearMessage: () => void;
  onLogout: () => void;
  onRefresh: () => Promise<void>;
  onSelectSection: (section: Section) => void;
  onSetDeploymentMode: (mode: DeploymentMode) => void;
  onToggleColorTheme: () => void;
}) {
  return (
    <div className={`admin-shell ${colorTheme === "dark" ? "dark theme-dark" : "theme-light"}`}>
      <aside className="sidebar">
        <SidebarBrand t={t} />
        <SidebarStatus deploymentMode={deploymentMode} serverInfo={serverInfo} serviceRunning={serviceRunning} t={t} />
        <SidebarNav section={section} t={t} onSelectSection={onSelectSection} />
        <SidebarFooter colorTheme={colorTheme} t={t} onLogout={onLogout} onToggleColorTheme={onToggleColorTheme} />
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
  return (
    <div className="sidebar-status-area">
      <div className="sidebar-server-card">
        <div className="sidebar-server-row">
          <StatusDot label={t(serviceRunning ? "running" : "stopped")} tone={serviceRunning ? "good" : "muted"} />
        </div>
        <div className="sidebar-address">{serverInfo?.serverUrl.replace("http://", "") ?? "127.0.0.1:3789"}</div>
        <div className="sidebar-deployment-label">{deploymentModeLabel(t, deploymentMode)}</div>
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

function SidebarFooter({ colorTheme, t, onLogout, onToggleColorTheme }: TranslatorContext & {
  colorTheme: ColorTheme;
  onLogout: () => void;
  onToggleColorTheme: () => void;
}) {
  return (
    <div className="sidebar-footer">
      <div className="sidebar-quick-actions">
        <button
          className={`sidebar-theme-switch${colorTheme === "dark" ? " is-dark" : ""}`}
          type="button"
          role="switch"
          aria-checked={colorTheme === "dark"}
          aria-label={t(colorTheme === "dark" ? "lightTheme" : "darkTheme")}
          onClick={onToggleColorTheme}
        >
          <span className="sidebar-theme-switch-icon is-light"><Sun size={12} /></span>
          <span className="sidebar-theme-switch-icon is-dark"><Moon size={12} /></span>
          <span className="sidebar-theme-switch-thumb">
            {colorTheme === "dark" ? <Moon size={12} /> : <Sun size={12} />}
          </span>
        </button>
        <button className="sidebar-logout-button" type="button" onClick={onLogout}>
          <LogOut size={15} />
          <span>{t("logout")}</span>
        </button>
      </div>
    </div>
  );
}
