import { Power } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { TranslatorContext } from "../../types";
import { DialogShell } from "./dialog-shell";

export function ShutdownServerDialog({
  busy,
  open,
  t,
  onClose,
  onConfirm
}: TranslatorContext & {
  busy: boolean;
  open: boolean;
  onClose: () => void;
  onConfirm: () => void | Promise<void>;
}) {
  return (
    <DialogShell
      className="shutdown-server-dialog"
      closeLabel={t("cancel")}
      open={open}
      subtitle={t("terminateServiceDialogHint")}
      title={t("terminateServiceConfirm")}
      titleId="shutdown-server-dialog-title"
      onClose={onClose}
    >
      <div className="shutdown-server-body">
        <Power aria-hidden="true" size={20} />
        <p>{t("terminateServiceHint")}</p>
      </div>
      <div className="dialog-footer">
        <Button disabled={busy} type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
        <Button disabled={busy} type="button" variant="destructive" onClick={() => void onConfirm()}>
          <Power size={15} />
          {busy ? t("serviceStopping") : t("terminateService")}
        </Button>
      </div>
    </DialogShell>
  );
}
