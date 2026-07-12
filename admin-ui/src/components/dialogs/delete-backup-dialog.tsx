import { Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { BackupRecord } from "../../api";
import type { TranslatorContext } from "../../types";
import { DialogShell } from "./dialog-shell";

export function DeleteBackupDialog({
  backup,
  busy,
  open,
  t,
  onClose,
  onConfirm
}: TranslatorContext & {
  backup: BackupRecord | null;
  busy: boolean;
  open: boolean;
  onClose: () => void;
  onConfirm: () => void | Promise<void>;
}) {
  return (
    <DialogShell
      className="delete-backup-dialog"
      closeLabel={t("cancel")}
      open={open}
      subtitle={t("deleteBackupDialogHint")}
      title={t("deleteBackupConfirm")}
      titleId="delete-backup-dialog-title"
      onClose={onClose}
    >
      <div className="delete-backup-body">
        <Trash2 aria-hidden="true" size={19} />
        <strong>{backup?.id ?? ""}</strong>
      </div>
      <div className="dialog-footer">
        <Button disabled={busy} type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
        <Button disabled={busy} type="button" variant="destructive" onClick={() => void onConfirm()}>
          <Trash2 size={15} />
          {t("delete")}
        </Button>
      </div>
    </DialogShell>
  );
}
