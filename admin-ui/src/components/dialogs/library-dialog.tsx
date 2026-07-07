import type { FormEvent } from "react";
import { Button } from "@/components/ui/button";
import type { TranslatorContext } from "../../types";
import { TextField } from "../common";
import { DialogShell } from "./dialog-shell";

export function LibraryDialog({
  open,
  title,
  hint,
  name,
  description,
  submitLabel,
  t,
  onClose,
  onDescriptionChange,
  onNameChange,
  onSubmit
}: TranslatorContext & {
  open: boolean;
  title: string;
  hint: string;
  name: string;
  description: string;
  submitLabel: string;
  onClose: () => void;
  onDescriptionChange: (value: string) => void;
  onNameChange: (value: string) => void;
  onSubmit: (event: FormEvent) => void | Promise<void>;
}) {
  return (
    <DialogShell
      className="library-dialog"
      closeLabel={t("cancel")}
      open={open}
      subtitle={hint}
      title={title}
      titleId="library-dialog-title"
      onClose={onClose}
    >
      <form className="dialog-form" onSubmit={onSubmit}>
        <div className="dialog-body">
          <TextField autoFocus required label={t("name")} value={name} onChange={onNameChange} />
          <TextField label={t("description")} value={description} onChange={onDescriptionChange} />
        </div>
        <div className="dialog-footer">
          <Button type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
          <Button type="submit">{submitLabel}</Button>
        </div>
      </form>
    </DialogShell>
  );
}
