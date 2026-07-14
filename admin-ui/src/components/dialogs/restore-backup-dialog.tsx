import { RotateCcw } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { BackupRecord } from "../../api";
import type { TranslatorContext } from "../../types";
import { DialogShell } from "./dialog-shell";

export function RestoreBackupDialog({
  backup,
  busy,
  open,
  sourceName,
  t,
  onClose,
  onConfirm
}: TranslatorContext & {
  backup: BackupRecord | null;
  busy: boolean;
  open: boolean;
  sourceName?: string;
  onClose: () => void;
  onConfirm: () => void | Promise<void>;
}) {
  return (
    <DialogShell
      className="restore-backup-dialog"
      closeLabel={t("cancel")}
      open={open}
      subtitle={t("restoreBackupDialogHint")}
      title={t("restoreBackupConfirm")}
      titleId="restore-backup-dialog-title"
      onClose={onClose}
    >
      <div className="restore-backup-body">
        <RotateCcw aria-hidden="true" size={20} />
        <div>
          <strong>{sourceName ?? backup?.id ?? ""}</strong>
          <p>{t("restoreBackupSafetyHint")}</p>
        </div>
      </div>
      <div className="dialog-footer">
        <Button disabled={busy} type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
        <Button disabled={busy} type="button" variant="destructive" onClick={() => void onConfirm()}>
          <RotateCcw size={15} />
          {busy ? t("restoreQueuing") : t("restore")}
        </Button>
      </div>
    </DialogShell>
  );
}
