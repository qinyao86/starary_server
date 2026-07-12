import { useState } from "react";
import { AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { TeamLibrary } from "../../api";
import type { TranslatorContext } from "../../types";
import { DialogShell } from "./dialog-shell";

export function DeleteLibraryDialog({
  library,
  open,
  t,
  onClose,
  onConfirm
}: TranslatorContext & {
  library: TeamLibrary | null;
  open: boolean;
  onClose: () => void;
  onConfirm: (deleteFiles: boolean) => void | Promise<void>;
}) {
  const [deleteFiles, setDeleteFiles] = useState(false);

  const close = () => {
    setDeleteFiles(false);
    onClose();
  };

  const submit = async () => {
    await onConfirm(deleteFiles);
    setDeleteFiles(false);
  };

  return (
    <DialogShell
      className="delete-library-dialog"
      closeLabel={t("cancel")}
      open={open}
      subtitle={t("deleteLibraryDialogHint")}
      title={t("deleteLibrary")}
      titleId="delete-library-dialog-title"
      onClose={close}
    >
      <div className="delete-library-body">
        <div className="delete-library-target">
          <AlertTriangle aria-hidden="true" size={18} />
          <strong>{library?.displayName ?? ""}</strong>
        </div>
        <label className="delete-library-files-option">
          <input
            checked={deleteFiles}
            type="checkbox"
            onChange={(event) => setDeleteFiles(event.target.checked)}
          />
          <span>
            <strong>{t("deleteLibraryFiles")}</strong>
            <small>{t("deleteLibraryFilesHint")}</small>
          </span>
        </label>
        <p className="delete-library-note">{t("deleteLibraryKeepFilesHint")}</p>
      </div>
      <div className="dialog-footer">
        <Button type="button" variant="outline" onClick={close}>{t("cancel")}</Button>
        <Button type="button" variant="destructive" onClick={() => void submit()}>{t("deleteLibrary")}</Button>
      </div>
    </DialogShell>
  );
}
