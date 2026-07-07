import { Database, Globe2, HardDrive, Play, Power, RefreshCw, Server, ShieldCheck, Square } from "lucide-react";
import type { PageContext } from "../types";
import { HealthRow, InfoStack, KeyValue, Panel, StatusDot } from "../components/common";
import { deploymentModeLabel, storageStatusLabel } from "../utils/format";

export function ServicePage({ t, deploymentMode, serviceRunning, serverInfo }: PageContext) {
  return (
    <div className="page-grid">
      <Panel title={t("serviceControl")} icon={Power} className="span-8">
        <div className="service-control">
          <div className={`service-orb${serviceRunning ? " is-on" : ""}`}>
            {serviceRunning ? <Power size={34} /> : <Square size={34} />}
          </div>
          <div className="service-lines">
            <StatusDot label={t(serviceRunning ? "running" : "stopped")} tone={serviceRunning ? "good" : "muted"} />
            <div className="endpoint-grid">
              <KeyValue label={t("serverUrl")} value={serverInfo?.serverUrl ?? "http://127.0.0.1:3789"} />
              <KeyValue label={t(deploymentMode === "local" ? "lanAddress" : "publicUrl")} value={deploymentMode === "local" ? "http://192.168.3.20:3789" : "https://team.example.com"} />
              <KeyValue label={t("apiVersion")} value={serverInfo?.apiVersion ?? "v1"} />
              <KeyValue label={t("mode")} value={deploymentModeLabel(t, deploymentMode)} />
            </div>
          </div>
          <div className="button-row vertical">
            <button className="primary-button is-disabled" type="button">
              {serviceRunning ? <Power size={16} /> : <Play size={16} />}
              <span>{t(serviceRunning ? "stopService" : "startService")}</span>
            </button>
            <button className="secondary-button is-disabled" type="button">
              <RefreshCw size={16} />
              <span>{t("restart")}</span>
            </button>
          </div>
        </div>
      </Panel>

      <Panel title={t("database")} icon={Database} className="span-4">
        <InfoStack
          items={[
            [t("postgresql"), deploymentMode === "local" ? t("running") : t("managedByDeployment")],
            [t("status"), serverInfo?.databaseStatus ?? t("connected")],
            [t("storageUsed"), t("plannedNote")]
          ]}
        />
      </Panel>

      <Panel title={t("healthChecks")} icon={ShieldCheck} className="span-12">
        <div className="health-line">
          <HealthRow icon={Server} label={t("serverProcess")} value={t("healthy")} tone="good" />
          <HealthRow icon={Database} label={t("database")} value={t("connected")} tone="good" />
          <HealthRow icon={HardDrive} label={t("storage")} value={storageStatusLabel(t, serverInfo)} tone={serverInfo?.storageWritable ? "good" : "warn"} />
          <HealthRow icon={Globe2} label={t("publicUrl")} value={deploymentMode === "cloud" ? t("notConfigured") : t("disabled")} tone="muted" />
        </div>
      </Panel>
    </div>
  );
}
