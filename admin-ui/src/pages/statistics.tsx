import { Activity, Database, HardDrive, Server } from "lucide-react";
import type { PageContext } from "../types";
import { ActivityList, MetricCard, PageFrame, Panel } from "../components/common";
import { formatBytes, formatCount, isUserOnline, storageStatusLabel } from "../utils/format";

export function StatisticsPage({ activityItems, libraries, serverInfo, serviceRunning, t, users }: PageContext) {
  const assetTotal = libraries.reduce((total, library) => total + (library.assetCount ?? 0), 0);
  const totalSize = libraries.reduce((total, library) => total + (library.totalSizeBytes ?? 0), 0);
  const onlineUsers = users.filter(isUserOnline).length;
  const resolveAvatarKey = (item: (typeof activityItems)[number]) => {
    const user = users.find((candidate) =>
      candidate.id === item.actorUserId ||
      candidate.email.toLowerCase() === item.actorEmail?.toLowerCase()
    );
    return user?.avatarKey ?? item.actorAvatarKey ?? null;
  };

  return (
    <PageFrame title={t("statistics")} description={t("statisticsMergedPageHint")}>
      <div className="metric-grid">
        <MetricCard label={t("teamLibraries")} value={formatCount(libraries.length)} change={t("realData")} tone="blue" />
        <MetricCard label={t("totalAssets")} value={formatCount(assetTotal)} change={t("realData")} tone="violet" />
        <MetricCard label={t("storageUsed")} value={formatBytes(totalSize)} change={t("realData")} tone="amber" />
        <MetricCard label={t("onlineUsers")} value={`${onlineUsers}/${users.length}`} change={t("realData")} tone="green" />
      </div>
      <div className="page-grid">
        <Panel title={t("healthChecks")} icon={Server} className="span-12">
          <div className="health-list">
            <HealthCheck icon={Server} label={t("serverProcess")} value={t(serviceRunning ? "running" : "stopped")} tone={serviceRunning ? "good" : "warn"} />
            <HealthCheck icon={Database} label={t("database")} value={serverInfo?.databaseStatus ?? "-"} tone={serverInfo?.databaseStatus === "connected" ? "good" : "muted"} />
            <HealthCheck icon={HardDrive} label={t("storage")} value={storageStatusLabel(t, serverInfo)} tone={serverInfo?.storageWritable ? "good" : "warn"} />
          </div>
        </Panel>
        <Panel title={t("recentActivity")} icon={Activity} className="span-12">
          <ActivityList t={t} activityItems={activityItems.slice(0, 16)} resolveAvatarKey={resolveAvatarKey} />
        </Panel>
      </div>
    </PageFrame>
  );
}

function HealthCheck({
  icon: Icon,
  label,
  tone,
  value
}: {
  icon: typeof Server;
  label: string;
  tone: "good" | "muted" | "warn";
  value: string;
}) {
  return (
    <div className={`health-row tone-${tone}`}>
      <span className="health-icon"><Icon size={17} /></span>
      <span className="health-label">{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
