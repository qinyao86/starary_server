import { Activity, Database, HardDrive, Server } from "lucide-react";
import type { PageContext } from "../types";
import { ActivityList, MetricCard, PageFrame, Panel } from "../components/common";
import { formatBytes, formatCount, isUserOnline, storageStatusLabel } from "../utils/format";

export function StatisticsPage({ activityItems, libraries, serverInfo, serviceRunning, t, users }: PageContext) {
  const assetTotal = libraries.reduce((total, library) => total + (library.assetCount ?? 0), 0);
  const totalSize = libraries.reduce((total, library) => total + (library.totalSizeBytes ?? 0), 0);
  const onlineUsers = users.filter(isUserOnline).length;

  return (
    <PageFrame title={t("statistics")} description={t("statisticsMergedPageHint")}>
      <div className="metric-grid">
        <MetricCard label={t("teamLibraries")} value={formatCount(libraries.length)} change={t("realData")} tone="blue" />
        <MetricCard label={t("totalAssets")} value={formatCount(assetTotal)} change={t("realData")} tone="violet" />
        <MetricCard label={t("storageUsed")} value={formatBytes(totalSize)} change={t("realData")} tone="amber" />
        <MetricCard label={t("onlineUsers")} value={`${onlineUsers}/${users.length}`} change={t("realData")} tone="green" />
      </div>
      <div className="page-grid">
        <Panel title={t("healthChecks")} icon={Server} className="span-5">
          <div className="health-list">
            <div className="health-row"><span><Server size={16} />{t("serverProcess")}</span><strong>{t(serviceRunning ? "running" : "stopped")}</strong></div>
            <div className="health-row"><span><Database size={16} />{t("database")}</span><strong>{serverInfo?.databaseStatus ?? "-"}</strong></div>
            <div className="health-row"><span><HardDrive size={16} />{t("storage")}</span><strong>{storageStatusLabel(t, serverInfo)}</strong></div>
          </div>
        </Panel>
        <Panel title={t("recentActivity")} icon={Activity} className="span-7">
          <ActivityList t={t} activityItems={activityItems.slice(0, 8)} />
        </Panel>
      </div>
    </PageFrame>
  );
}
