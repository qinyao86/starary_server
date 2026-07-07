import { Server, Settings } from "lucide-react";
import type { CurrentUser, ServerInfo } from "../api";
import type { DeploymentMode, TranslatorContext } from "../types";
import { InfoStack, Panel } from "../components/common";
import { deploymentModeLabel, roleLabel, storageStatusLabel } from "../utils/format";

export function SettingsPage({ t, deploymentMode, serverInfo, currentUser }: TranslatorContext & { deploymentMode: DeploymentMode; serverInfo: ServerInfo | null; currentUser: CurrentUser | null }) {
  return (
    <div className="page-grid">
      <Panel title={t("settings")} icon={Settings} className="span-6">
        <InfoStack
          items={[
            [t("deployment"), deploymentModeLabel(t, deploymentMode)],
            [t("license"), "Team Prototype"],
            [t("updateChannel"), "Beta"],
            [t("logs"), "info"]
          ]}
        />
      </Panel>
      <Panel title={t("serverInfo")} icon={Server} className="span-6">
        <InfoStack
          items={[
            [t("signedInAs"), currentUser ? `${currentUser.displayName} (${roleLabel(t, currentUser.role)})` : "-"],
            [t("serverUrl"), serverInfo?.serverUrl ?? "-"],
            [t("serverStorageDir"), serverInfo?.storageDir ?? "-"],
            [t("storage"), storageStatusLabel(t, serverInfo)],
            [t("adminAssets"), serverInfo?.adminAvailable ? t("configured") : t("notConfigured")]
          ]}
        />
      </Panel>
    </div>
  );
}
