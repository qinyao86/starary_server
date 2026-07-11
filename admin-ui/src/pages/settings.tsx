import { Moon, Palette, Server, Settings, Sun } from "lucide-react";
import type { PageContext } from "../types";
import { InfoStack, PageFrame, Panel, Segmented } from "../components/common";
import { deploymentModeLabel, roleLabel, storageStatusLabel } from "../utils/format";

export function SettingsPage({ t, colorTheme, setColorTheme, deploymentMode, serverInfo, currentUser }: PageContext) {
  return (
    <PageFrame title={t("settings")} description={t("settingsPageHint")}>
      <div className="page-grid">
        <Panel title={t("appearance")} icon={Palette} className="span-12">
          <div className="appearance-setting">
            <div>
              <strong>{t("interfaceTheme")}</strong>
              <span>{t("interfaceThemeHint")}</span>
            </div>
            <Segmented
              value={colorTheme}
              options={[
                { value: "light", label: t("lightTheme"), icon: Sun },
                { value: "dark", label: t("darkTheme"), icon: Moon }
              ]}
              onChange={(value) => setColorTheme(value === "light" ? "light" : "dark")}
            />
          </div>
        </Panel>
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
    </PageFrame>
  );
}
