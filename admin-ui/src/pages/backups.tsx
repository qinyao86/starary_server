import { Archive, Lock, RefreshCw } from "lucide-react";
import type { DeploymentMode, TranslatorContext } from "../types";
import { InfoStack, Panel } from "../components/common";

export function BackupsPage({ t, deploymentMode }: TranslatorContext & { deploymentMode: DeploymentMode }) {
  return (
    <div className="page-grid">
      <Panel title={t("backups")} icon={Archive} className="span-7" action={<button className="primary-button is-disabled" type="button">{t("backupNow")}</button>}>
        <InfoStack
          items={[
            [t("lastBackup"), t("plannedNote")],
            [t("backupTarget"), deploymentMode === "local" ? "E:\\MadLibraryBackups" : "s3://madlibrary-backups"],
            [t("status"), t("prototype")]
          ]}
        />
      </Panel>
      <Panel title={t("restore")} icon={RefreshCw} className="span-5">
        <div className="placeholder-box">
          <Lock size={18} />
          <span>{t("plannedNote")}</span>
        </div>
      </Panel>
    </div>
  );
}
