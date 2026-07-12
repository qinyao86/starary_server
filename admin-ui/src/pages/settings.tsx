import { useEffect, useState, type FormEvent } from "react";
import { Moon, Palette, Power, Server, Settings, Sun } from "lucide-react";
import { Button } from "@/components/ui/button";
import { api, type RuntimeSettings } from "../api";
import { ShutdownServerDialog } from "../components/dialogs";
import type { PageContext } from "../types";
import { InfoStack, PageFrame, Panel, Segmented } from "../components/common";
import { canManageServerRole, deploymentModeLabel, roleLabel, storageStatusLabel } from "../utils/format";

export function SettingsPage({ t, colorTheme, setColorTheme, deploymentMode, serverInfo, currentUser, token, setMessage }: PageContext) {
  const [runtimeSettings, setRuntimeSettings] = useState<RuntimeSettings | null>(null);
  const [port, setPort] = useState("");
  const [savingPort, setSavingPort] = useState(false);
  const [shutdownOpen, setShutdownOpen] = useState(false);
  const [shuttingDown, setShuttingDown] = useState(false);
  const canManageRuntime = Boolean(token && canManageServerRole(currentUser?.role ?? ""));

  useEffect(() => {
    if (!canManageRuntime) return;
    void api.runtimeSettings(token).then((settings) => {
      setRuntimeSettings(settings);
      setPort(String(settings.configuredPort));
    }).catch((error) => setMessage(error instanceof Error ? error.message : String(error)));
  }, [canManageRuntime, setMessage, token]);

  const savePort = async (event: FormEvent) => {
    event.preventDefault();
    const nextPort = Number(port);
    if (!Number.isInteger(nextPort) || nextPort < 1024 || nextPort > 65535) {
      setMessage(t("invalidServerPort"));
      return;
    }
    setSavingPort(true);
    try {
      const settings = await api.updateRuntimeSettings(token, { port: nextPort });
      setRuntimeSettings(settings);
      setPort(String(settings.configuredPort));
      setMessage(t("portSaved"));
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setSavingPort(false);
    }
  };

  const shutdown = async () => {
    setShuttingDown(true);
    try {
      await api.shutdownServer(token);
      setMessage(t("serviceStopping"));
      setShutdownOpen(false);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
      setShuttingDown(false);
    }
  };

  return (
    <PageFrame title={t("settings")} description={t("settingsPageHint")}>
      <ShutdownServerDialog
        busy={shuttingDown}
        open={shutdownOpen}
        t={t}
        onClose={() => !shuttingDown && setShutdownOpen(false)}
        onConfirm={shutdown}
      />
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
              [t("lanAddress"), serverInfo?.lanAdminUrl ?? t("lanAddressUnavailable")],
              [t("localAddress"), serverInfo?.localAdminUrl ?? "-"],
              [t("serverStorageDir"), serverInfo?.storageDir ?? "-"],
              [t("storage"), storageStatusLabel(t, serverInfo)],
              [t("adminAssets"), serverInfo?.adminAvailable ? t("configured") : t("notConfigured")]
            ]}
          />
        </Panel>
        {canManageRuntime && (
          <Panel title={t("serverRuntime")} icon={Server} className="span-12">
            <div className="runtime-settings">
              {runtimeSettings?.serviceControlAvailable ? (
                <form className="runtime-port-form" onSubmit={savePort}>
                  <div className="runtime-setting-copy">
                    <strong>{t("serverPort")}</strong>
                    <span>{t("serverPortHint")}</span>
                    {runtimeSettings.restartRequired && <em>{t("restartRequired")}</em>}
                  </div>
                  <label className="field runtime-port-field">
                    <span>{t("currentPort")}: {runtimeSettings.currentPort}</span>
                    <input
                      aria-label={t("serverPort")}
                      inputMode="numeric"
                      max="65535"
                      min="1024"
                      type="number"
                      value={port}
                      onChange={(event) => setPort(event.target.value)}
                    />
                  </label>
                  <Button disabled={savingPort || !port} size="sm" type="submit">{t("save")}</Button>
                </form>
              ) : (
                <p className="runtime-unavailable">{t("serviceControlsUnavailable")}</p>
              )}
              <div className="runtime-danger-row">
                <div className="runtime-setting-copy">
                  <strong>{t("terminateService")}</strong>
                  <span>{t("terminateServiceHint")}</span>
                  <small>{t("accountLogoutHint")}</small>
                </div>
                <Button
                  disabled={!runtimeSettings?.serviceControlAvailable}
                  size="sm"
                  type="button"
                  variant="destructive"
                  onClick={() => setShutdownOpen(true)}
                >
                  <Power size={15} />
                  {t("terminateService")}
                </Button>
              </div>
            </div>
          </Panel>
        )}
      </div>
    </PageFrame>
  );
}
