import type { FormEvent } from "react";
import { Button } from "@/components/ui/button";
import type { StorageConnection } from "../../api";
import type { TranslatorContext } from "../../types";
import { StorageLocationFields } from "../storage/storage-location-fields";
import { DialogShell } from "./dialog-shell";

export function StorageConnectionDialog({
  connection,
  kind,
  location,
  open,
  subtitle,
  t,
  title,
  onClose,
  onKindChange,
  onLocationChange,
  onSubmit
}: TranslatorContext & {
  connection: StorageConnection | null;
  kind: string;
  location: string;
  open: boolean;
  subtitle?: string;
  title?: string;
  onClose: () => void;
  onKindChange: (kind: string) => void;
  onLocationChange: (location: string) => void;
  onSubmit: (event: FormEvent) => void | Promise<void>;
}) {
  return (
    <DialogShell
      className="storage-connection-dialog"
      closeLabel={t("cancel")}
      open={open}
      subtitle={subtitle ?? t("storageConnectionDialogHint")}
      title={title ?? (connection ? t("editStorageConnection") : t("createStorageConnection"))}
      titleId="storage-connection-dialog-title"
      onClose={onClose}
    >
      <form className="dialog-form" onSubmit={onSubmit}>
        <div className="dialog-body">
          <StorageLocationFields kind={kind} location={location} t={t} onKindChange={onKindChange} onLocationChange={onLocationChange} />
        </div>
        <div className="dialog-footer">
          <Button type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
          <Button type="submit">{t("save")}</Button>
        </div>
      </form>
    </DialogShell>
  );
}
