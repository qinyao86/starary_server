import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  Cloud,
  Database,
  HardDrive,
  Library,
  Play,
  Power,
  RefreshCw,
  Server,
  ShieldCheck,
  Users
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { PageContext } from "../types";
import { ActivityList, DataTable, HealthRow, InfoStack, Panel } from "../components/common";
import { deploymentModeLabel, formatBytes, formatCount, formatDateTime, isUserOnline, roleLabel, storageStatusLabel } from "../utils/format";

type SignalTone = "good" | "warn" | "muted";

function OverviewSignalCard({
  icon: Icon,
  title,
  value,
  summary,
  tone,
  details
}: {
  icon: LucideIcon;
  title: string;
  value: string;
  summary: string;
  tone: SignalTone;
  details: Array<[string, string]>;
}) {
  return (
    <article className={`overview-signal-card tone-${tone}`}>
      <div className="overview-signal-top">
        <span className="overview-signal-icon"><Icon size={19} /></span>
        <span className="overview-signal-status" />
      </div>
      <div>
        <div className="overview-signal-title">{title}</div>
        <div className="overview-signal-value">{value}</div>
        <p>{summary}</p>
      </div>
      <div className="overview-signal-details">
        {details.map(([label, detail]) => (
          <div key={label}>
            <span>{label}</span>
            <strong>{detail}</strong>
          </div>
        ))}
      </div>
    </article>
  );
}

export function OverviewPage({
  t,
  deploymentMode,
  serviceRunning,
  serverInfo,
  libraries,
  users,
  assetTotal,
  activityItems,
  storageRoots,
  selectedLibrary,
  refreshAll,
  setMessage
}: PageContext) {
  const onlineUserCount = users.filter((item) => isUserOnline(item)).length;
  const enabledStorageRoots = storageRoots.filter((item) => item.enabled).length;
  const disabledStorageRoots = storageRoots.length - enabledStorageRoots;
  const totalLibraryAssets = libraries.reduce((total, library) => total + (library.assetCount ?? 0), 0);
  const knownAssetTotal = totalLibraryAssets || assetTotal;
  const totalLibrarySize = libraries.reduce((total, library) => total + (library.totalSizeBytes ?? 0), 0);
  const databaseReady = serverInfo?.databaseStatus === "connected";
  const storageReady = Boolean(serverInfo?.storageWritable);
  const serviceReady = serviceRunning && databaseReady && storageReady;
  const storageTone: SignalTone = storageReady ? "good" : "warn";
  const serviceTone: SignalTone = serviceReady ? "good" : serviceRunning ? "warn" : "muted";
  const libraryTone: SignalTone = libraries.length > 0 ? "good" : "warn";
  const latestActivity = activityItems[0];
  const selectedLibraryLabel = selectedLibrary?.displayName ?? t("noSelectedLibrary");

  return (
    <div className="page-grid overview-dashboard">
      <section className={`overview-hero tone-${serviceReady ? "good" : "warn"}`}>
        <div className="overview-hero-main">
          <span className="overview-hero-orb">{serviceReady ? <CheckCircle2 size={28} /> : <AlertTriangle size={28} />}</span>
          <div>
            <span className="overview-eyebrow">{t("overviewLiveStatus")}</span>
            <h2>{serviceRunning ? t("running") : t("stopped")}</h2>
            <p>{serviceReady ? t("overviewReadySummary") : t("overviewAttentionSummary")}</p>
          </div>
        </div>
        <div className="overview-hero-facts">
          <div>
            <span>{t("serverUrl")}</span>
            <strong>{serverInfo?.serverUrl ?? "-"}</strong>
          </div>
          <div>
            <span>{t("database")}</span>
            <strong>{databaseReady ? t("connected") : serverInfo?.databaseStatus ?? "-"}</strong>
          </div>
          <div>
            <span>{t("storage")}</span>
            <strong>{storageStatusLabel(t, serverInfo)}</strong>
          </div>
        </div>
        <div className="overview-hero-actions">
          <button
            aria-disabled="true"
            aria-pressed={serviceRunning}
            className={`overview-power-switch ${serviceRunning ? "is-on" : ""}`}
            type="button"
            onClick={() => setMessage(t("serviceControlUnavailable"))}
          >
            <span className="overview-power-switch-track">
              <span className="overview-power-switch-thumb">
                {serviceRunning ? <Power size={20} /> : <Play size={20} />}
              </span>
            </span>
            <span className="overview-power-switch-text">
              <strong>{t(serviceRunning ? "stopService" : "startService")}</strong>
              <small>{t("serviceManagedExternally")}</small>
            </span>
          </button>
          <button className="overview-refresh-button" type="button" onClick={() => void refreshAll()}>
            <RefreshCw size={15} />
            {t("refresh")}
          </button>
        </div>
      </section>

      <div className="overview-signal-grid">
        <OverviewSignalCard
          icon={Server}
          title={t("overviewServiceCard")}
          value={serviceRunning ? t("running") : t("stopped")}
          summary={serverInfo?.serverUrl ?? t("notConfigured")}
          tone={serviceTone}
          details={[
            [t("deployment"), deploymentModeLabel(t, deploymentMode)],
            [t("apiVersion"), serverInfo?.apiVersion ?? "-"],
            [t("adminAssets"), serverInfo?.adminAvailable ? t("configured") : t("notConfigured")]
          ]}
        />
        <OverviewSignalCard
          icon={Library}
          title={t("overviewLibrariesCard")}
          value={formatCount(libraries.length)}
          summary={`${t("knownAssets")}: ${formatCount(knownAssetTotal)} / ${t("knownSize")}: ${formatBytes(totalLibrarySize)}`}
          tone={libraryTone}
          details={[
            [t("onlineUsers"), formatCount(onlineUserCount)],
            [t("latestEvent"), latestActivity ? formatDateTime(latestActivity.createdAt) : t("empty")],
            [t("selectedLibrary"), selectedLibraryLabel]
          ]}
        />
        <OverviewSignalCard
          icon={deploymentMode === "local" ? HardDrive : Cloud}
          title={t("overviewStorageCard")}
          value={storageStatusLabel(t, serverInfo)}
          summary={serverInfo?.storageDir ?? "-"}
          tone={storageTone}
          details={[
            [t("currentStorageRoots"), formatCount(storageRoots.length)],
            [t("enabledRoots"), formatCount(enabledStorageRoots)],
            [t("disabledRoots"), formatCount(disabledStorageRoots)]
          ]}
        />
      </div>

      <Panel title={t("healthChecks")} icon={ShieldCheck} className="span-7">
        <div className="status-grid">
          <HealthRow icon={Server} label={t("serverProcess")} value={t(serviceRunning ? "running" : "stopped")} tone={serviceRunning ? "good" : "muted"} />
          <HealthRow icon={Database} label={t("database")} value={databaseReady ? t("connected") : serverInfo?.databaseStatus ?? "-"} tone={databaseReady ? "good" : "warn"} />
          <HealthRow icon={deploymentMode === "local" ? HardDrive : Cloud} label={deploymentMode === "local" ? t("localStorage") : t("objectStorage")} value={storageStatusLabel(t, serverInfo)} tone={storageReady ? "good" : "warn"} />
          <HealthRow icon={Users} label={t("onlineUsers")} value={formatCount(onlineUserCount)} tone={onlineUserCount > 0 ? "good" : "muted"} />
          <HealthRow icon={Library} label={t("selectedLibrary")} value={selectedLibraryLabel} tone={selectedLibrary ? "good" : "muted"} />
        </div>
      </Panel>

      <Panel title={t("recentActivity")} icon={Activity} className="span-5">
        <ActivityList t={t} activityItems={activityItems} compact />
      </Panel>

      <Panel title={t("libraries")} icon={Library} className="span-7">
        <DataTable
          emptyLabel={t("noLibraries")}
          columns={[t("libraryName"), t("role"), t("description")]}
          rows={libraries.slice(0, 4).map((item) => [item.displayName, roleLabel(t, item.currentUserRole ?? "owner"), item.description ?? ""])}
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
