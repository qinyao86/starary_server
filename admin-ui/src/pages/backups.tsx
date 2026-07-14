import { useCallback, useEffect, useRef, useState, type ChangeEvent, type FormEvent } from "react";
import { Archive, Clock3, Download, RefreshCw, RotateCcw, Trash2, Upload } from "lucide-react";
import { Button } from "@/components/ui/button";
import { api, clearStoredToken, type BackupRecord, type BackupStatus } from "../api";
import { PageFrame, Panel } from "../components/common";
import { DeleteBackupDialog, InitializeServerDialog, RestoreBackupDialog } from "../components/dialogs";
import type { PageContext } from "../types";
import { formatBytes } from "../utils/format";

export function BackupsPage({ t, token, currentUser, setMessage, resetAfterInitialization }: PageContext) {
  const [status, setStatus] = useState<BackupStatus | null>(null);
  const [backups, setBackups] = useState<BackupRecord[]>([]);
  const [automaticEnabled, setAutomaticEnabled] = useState(true);
  const [automaticTime, setAutomaticTime] = useState("02:00");
  const [retentionCount, setRetentionCount] = useState("30");
  const [busy, setBusy] = useState(false);
  const [restoreBackup, setRestoreBackup] = useState<BackupRecord | null>(null);
  const [restoreBusy, setRestoreBusy] = useState(false);
  const [restoreFile, setRestoreFile] = useState<File | null>(null);
  const [restoreFileBusy, setRestoreFileBusy] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<BackupRecord | null>(null);
  const [initializeOpen, setInitializeOpen] = useState(false);
  const [initializeBusy, setInitializeBusy] = useState(false);
  const restoreFileInputRef = useRef<HTMLInputElement | null>(null);

  const load = useCallback(async () => {
    const overview = await api.backupOverview(token);
    setStatus(overview.status);
    setBackups(overview.backups);
    setAutomaticEnabled(overview.status.settings.automaticEnabled);
    setAutomaticTime(overview.status.settings.automaticTime);
    setRetentionCount(String(overview.status.settings.retentionCount));
  }, [token]);

  useEffect(() => {
    void load().catch((error) => setMessage(error instanceof Error ? error.message : String(error)));
  }, [load, setMessage]);

  const savePolicy = async (event: FormEvent) => {
    event.preventDefault();
    const retention = Number(retentionCount);
    if (!Number.isInteger(retention) || retention < 1 || retention > 365) {
      setMessage(t("invalidBackupRetention"));
      return;
    }
    setBusy(true);
    try {
      const nextStatus = await api.updateBackupSettings(token, {
        automaticEnabled,
        automaticTime,
        retentionCount: retention
      });
      setStatus(nextStatus);
      setMessage(t("backupPolicySaved"));
      await load();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  const createBackup = async () => {
    setBusy(true);
    try {
      await api.createBackup(token);
      setMessage(t("backupCreated"));
      await load();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  const downloadBackup = async (backup: BackupRecord) => {
    try {
      const blob = await api.downloadBackup(token, backup.id);
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = backup.id;
      anchor.click();
      URL.revokeObjectURL(url);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const deleteBackup = async () => {
    if (!deleteTarget) return;
    setBusy(true);
    try {
      await api.deleteBackup(token, deleteTarget.id);
      setDeleteTarget(null);
      setMessage(t("backupDeleted"));
      await load();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  const confirmRestore = async () => {
    if (!restoreBackup) return;
    setRestoreBusy(true);
    try {
      await api.restoreBackup(token, restoreBackup.id);
      setMessage(t("restoreQueued"));
      setRestoreBackup(null);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
      setRestoreBusy(false);
    }
  };

  const selectRestoreFile = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0] ?? null;
    event.target.value = "";
    if (!file) return;
    if (!file.name.toLowerCase().endsWith(".dump")) {
      setMessage(t("invalidBackupFile"));
      return;
    }
    setRestoreFile(file);
  };

  const confirmRestoreFile = async () => {
    if (!restoreFile) return;
    setRestoreFileBusy(true);
    try {
      await api.restoreBackupFile(token, restoreFile);
      setMessage(t("restoreQueued"));
      setRestoreFile(null);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
      setRestoreFileBusy(false);
    }
  };

  const initializeServer = async () => {
    setInitializeBusy(true);
    try {
      await api.initializeServer(token);
      clearStoredToken();
      await resetAfterInitialization();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
      setInitializeBusy(false);
    }
  };

  const latestBackup = backups[0];

  return (
    <PageFrame
      title={t("data")}
      description={t("dataPageHint")}
      action={
        <>
          <input
            ref={restoreFileInputRef}
            accept=".dump"
            style={{ display: "none" }}
            type="file"
            onChange={selectRestoreFile}
          />
          <Button disabled={busy || restoreBusy || restoreFileBusy || !status?.available} type="button" variant="outline" onClick={() => restoreFileInputRef.current?.click()}>
            <Upload size={16} />
            {t("restoreFromFile")}
          </Button>
          <Button disabled={busy || !status?.available} type="button" onClick={() => void createBackup()}>
            <Archive size={16} />
            {busy ? t("backupWorking") : t("backupNow")}
          </Button>
        </>
      }
    >
      <RestoreBackupDialog
        backup={restoreBackup}
        busy={restoreBusy}
        open={Boolean(restoreBackup)}
        t={t}
        onClose={() => !restoreBusy && setRestoreBackup(null)}
        onConfirm={confirmRestore}
      />
      <RestoreBackupDialog
        backup={null}
        busy={restoreFileBusy}
        open={Boolean(restoreFile)}
        sourceName={restoreFile?.name}
        t={t}
        onClose={() => !restoreFileBusy && setRestoreFile(null)}
        onConfirm={confirmRestoreFile}
      />
      <DeleteBackupDialog
        backup={deleteTarget}
        busy={busy}
        open={Boolean(deleteTarget)}
        t={t}
        onClose={() => !busy && setDeleteTarget(null)}
        onConfirm={deleteBackup}
      />
      <InitializeServerDialog
        busy={initializeBusy}
        open={initializeOpen}
        t={t}
        onClose={() => !initializeBusy && setInitializeOpen(false)}
        onConfirm={initializeServer}
      />
      <div className="backup-summary-grid">
        <div className="backup-summary-item">
          <Clock3 size={17} />
          <span>{t("automaticBackup")}</span>
          <strong>{automaticEnabled ? automaticTime : t("disabled")}</strong>
        </div>
        <div className="backup-summary-item">
          <Archive size={17} />
          <span>{t("backupFiles")}</span>
          <strong>{backups.length}</strong>
        </div>
        <div className="backup-summary-item">
          <RefreshCw size={17} />
          <span>{t("lastBackup")}</span>
          <strong>{latestBackup ? formatBackupTime(latestBackup.createdAt) : t("empty")}</strong>
        </div>
      </div>
      <div className="page-grid backup-page-grid">
        <Panel title={t("automaticBackup")} icon={Clock3} className="span-12">
          <form className="backup-policy-form" onSubmit={savePolicy}>
            <div className="backup-policy-copy">
              <strong>{t("dailyBackup")}</strong>
              <span>{t("dailyBackupHint")}</span>
            </div>
            <label className="backup-toggle">
              <input aria-label={t("automaticBackup")} checked={automaticEnabled} type="checkbox" onChange={(event) => setAutomaticEnabled(event.target.checked)} />
              <span aria-hidden="true" />
            </label>
            <label className="field">
              <span>{t("backupTime")}</span>
              <input disabled={!automaticEnabled} required type="time" value={automaticTime} onChange={(event) => setAutomaticTime(event.target.value)} />
            </label>
            <label className="field">
              <span>{t("maximumBackups")}</span>
              <input disabled={!automaticEnabled} max="365" min="1" required type="number" value={retentionCount} onChange={(event) => setRetentionCount(event.target.value)} />
            </label>
            <Button disabled={busy} size="sm" type="submit">{t("save")}</Button>
          </form>
          <div className="backup-path-row">
            <span>{t("backupTarget")}</span>
            <code title={status?.backupDir}>{status?.backupDir ?? "-"}</code>
          </div>
        </Panel>
        <Panel title={t("backupHistory")} icon={Archive} className="span-12">
          {backups.length ? (
            <div className="backup-list">
              {backups.map((backup) => (
                <div className="backup-row" key={backup.id}>
                  <Archive aria-hidden="true" size={17} />
                  <div className="backup-row-main">
                    <strong>{backup.id}</strong>
                    <span>{formatBackupTime(backup.createdAt)} / {formatBytes(backup.sizeBytes)}</span>
                  </div>
                  <span className={`backup-kind is-${backup.kind}`}>{backupKindLabel(t, backup.kind)}</span>
                  <div className="backup-actions">
                    <Button aria-label={t("downloadBackup")} title={t("downloadBackup")} size="icon" type="button" variant="ghost" onClick={() => void downloadBackup(backup)}>
                      <Download size={15} />
                    </Button>
                    <Button aria-label={t("restore")} title={t("restore")} size="icon" type="button" variant="ghost" onClick={() => setRestoreBackup(backup)}>
                      <RotateCcw size={15} />
                    </Button>
                    <Button aria-label={t("delete")} title={t("delete")} size="icon" type="button" variant="ghost" onClick={() => setDeleteTarget(backup)}>
                      <Trash2 size={15} />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="placeholder-box">{status?.available ? t("noBackups") : t("backupToolsUnavailable")}</div>
          )}
        </Panel>
        {currentUser?.role === "owner" && (
          <Panel title={t("initializeServer")} icon={RotateCcw} className="span-12">
            <div className="data-danger-row">
              <div>
                <strong>{t("initializeServer")}</strong>
                <span>{t("initializeServerHint")}</span>
              </div>
              <Button type="button" variant="destructive" onClick={() => setInitializeOpen(true)}>
                <RotateCcw size={15} />
                {t("initializeServer")}
              </Button>
            </div>
          </Panel>
        )}
      </div>
    </PageFrame>
  );
}

function formatBackupTime(value: string) {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short"
  }).format(new Date(value));
}

function backupKindLabel(t: PageContext["t"], kind: BackupRecord["kind"]) {
  if (kind === "automatic") return t("automatic");
  if (kind === "pre_restore") return t("preRestoreBackup");
  if (kind === "pre_initialize") return t("preInitializeBackup");
  return t("manual");
}
