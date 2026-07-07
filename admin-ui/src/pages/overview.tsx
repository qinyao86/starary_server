import { Activity, Cloud, Database, HardDrive, Library, Network, RefreshCw, Server, ShieldCheck } from "lucide-react";
import type { PageContext } from "../types";
import { ActivityList, DataTable, HealthRow, InfoStack, MetricCard, Panel } from "../components/common";
import { deploymentModeLabel, isUserOnline, roleLabel, storageStatusLabel } from "../utils/format";

export function OverviewPage({ t, deploymentMode, serviceRunning, serverInfo, libraries, users, assetTotal, activityItems, storageRoots }: PageContext) {
  const onlineUserCount = users.filter((item) => isUserOnline(item)).length;
  const cards = [
    { key: "activeUsers", value: String(onlineUserCount), change: t("realData"), tone: "blue" },
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
