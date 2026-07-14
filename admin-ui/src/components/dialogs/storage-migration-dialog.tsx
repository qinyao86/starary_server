import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import { AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { StorageConnection } from "../../api";
import type { TranslatorContext } from "../../types";
import { formatBytes, formatCount } from "../../utils/format";
import { StorageLocationFields } from "../storage/storage-location-fields";
import { DialogShell } from "./dialog-shell";

export function StorageMigrationDialog({
  busy,
  connection,
  kind,
  location,
  open,
  t,
  onClose,
  onKindChange,
  onLocationChange,
  onSubmit
}: TranslatorContext & {
  busy: boolean;
  connection: StorageConnection | null;
  kind: string;
  location: string;
  open: boolean;
  onClose: () => void;
  onKindChange: (kind: string) => void;
  onLocationChange: (location: string) => void;
  onSubmit: (event: FormEvent) => void | Promise<void>;
}) {
  const [acknowledged, setAcknowledged] = useState(false);

  useEffect(() => {
    if (open) setAcknowledged(false);
  }, [open, connection?.id]);

  return (
    <DialogShell
      className="storage-migration-dialog"
      closeLabel={t("cancel")}
      open={open}
      subtitle={t("storageMigrationDialogHint")}
      title={t("migrateStorageConnection")}
      titleId="storage-migration-dialog-title"
      onClose={() => { if (!busy) onClose(); }}
    >
      <form className="dialog-form" onSubmit={onSubmit}>
        <div className="dialog-body storage-migration-body">
          <div className="storage-migration-current">
            <span>{t("currentStorageLocation")}</span>
            <strong>{connection?.canonicalUri ?? "-"}</strong>
          </div>
          <div className="storage-migration-summary">
            <div><span>{t("linkedLibraries")}</span><strong>{formatCount(connection?.libraryCount)}</strong></div>
            <div><span>{t("assets")}</span><strong>{formatCount(connection?.assetCount)}</strong></div>
            <div><span>{t("estimatedDataSize")}</span><strong>{formatBytes(connection?.totalSizeBytes)}</strong></div>
          </div>
          {connection?.libraryNames.length ? (
            <div className="storage-migration-libraries">
              <span>{t("affectedLibraries")}</span>
              <p>{connection.libraryNames.join(", ")}</p>
            </div>
          ) : null}
          <StorageLocationFields
            allowedKinds={["server_filesystem", "smb"]}
            kind={kind}
            location={location}
            locationLabel={t("migrationDestination")}
            t={t}
            onKindChange={onKindChange}
            onLocationChange={onLocationChange}
          />
          <div className="storage-migration-warning">
            <AlertTriangle aria-hidden="true" size={17} />
            <div>
              <strong>{t("migrationCopiesDataAndFiles")}</strong>
              <p>{t("migrationKeepsOldFiles")}</p>
            </div>
          </div>
          <label className="storage-migration-acknowledgement">
            <input
              checked={acknowledged}
              disabled={busy}
              type="checkbox"
              onChange={(event) => setAcknowledged(event.target.checked)}
            />
            <span>{t("migrationAcknowledgement")}</span>
          </label>
        </div>
        <div className="dialog-footer">
          <Button disabled={busy} type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
          <Button disabled={!acknowledged || busy || !location.trim()} type="submit">
            {busy ? t("migratingStorage") : t("startMigration")}
          </Button>
        </div>
      </form>
    </DialogShell>
  );
}
