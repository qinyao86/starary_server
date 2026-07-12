import { RotateCcw } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { TranslatorContext } from "../../types";
import { DialogShell } from "./dialog-shell";

export function InitializeServerDialog({ busy, open, t, onClose, onConfirm }: TranslatorContext & {
  busy: boolean;
  open: boolean;
  onClose: () => void;
  onConfirm: () => void | Promise<void>;
}) {
  return (
    <DialogShell
      className="initialize-server-dialog"
      closeLabel={t("cancel")}
      open={open}
      subtitle={t("initializeServerDialogHint")}
      title={t("initializeServerConfirm")}
      titleId="initialize-server-dialog-title"
      onClose={onClose}
    >
      <div className="initialize-server-body">
        <RotateCcw aria-hidden="true" size={20} />
        <div>
          <strong>{t("initializeServerWarning")}</strong>
          <p>{t("initializeServerSafetyHint")}</p>
        </div>
      </div>
      <div className="dialog-footer">
        <Button disabled={busy} type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
        <Button disabled={busy} type="button" variant="destructive" onClick={() => void onConfirm()}>
          <RotateCcw size={15} />
          {busy ? t("initializingServer") : t("initializeServer")}
        </Button>
      </div>
    </DialogShell>
  );
}
