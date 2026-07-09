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
  onSubmit,
  showWorkspaceSection = false,
  workspaceCanonicalUri = "",
  workspaceKind = "smb",
  onWorkspaceCanonicalUriChange,
  onWorkspaceKindChange
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
  showWorkspaceSection?: boolean;
  workspaceCanonicalUri?: string;
  workspaceKind?: string;
  onWorkspaceCanonicalUriChange?: (value: string) => void;
  onWorkspaceKindChange?: (value: string) => void;
}) {
  const canEditWorkspace =
    showWorkspaceSection &&
    onWorkspaceCanonicalUriChange &&
    onWorkspaceKindChange;
  const storageKindOptions = [
    { value: "smb", label: t("storageKindSmb"), description: t("sharedFolderTypeHint") },
    { value: "s3", label: t("storageKindS3"), description: t("objectStorageTypeHint") }
  ];
  const handleWorkspaceKindChange = (value: string) => onWorkspaceKindChange?.(value);
  const handleWorkspaceCanonicalUriChange = (value: string) => onWorkspaceCanonicalUriChange?.(value);

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
          {canEditWorkspace && (
            <div className="library-storage-setup">
              <div className="library-storage-heading">
                <strong>{t("libraryStorageLocation")}</strong>
                <span>{t("libraryStorageLocationHint")}</span>
              </div>
              <div className="storage-kind-options" role="radiogroup" aria-label={t("storageLocationType")}>
                {storageKindOptions.map((option) => (
                  <button
                    aria-checked={workspaceKind === option.value}
                    className={`storage-kind-option${workspaceKind === option.value ? " is-active" : ""}`}
                    key={option.value}
                    role="radio"
                    type="button"
                    onClick={() => handleWorkspaceKindChange(option.value)}
                  >
                    <span className="storage-kind-option-title">{option.label}</span>
                    <span className="storage-kind-option-description">{option.description}</span>
                  </button>
                ))}
              </div>
              <div className="library-storage-fields">
                <TextField
                  required
                  label={t("workspaceLocation")}
                  placeholder={t(workspaceKind === "s3" ? "objectStorageLocationPlaceholder" : "sharedFolderLocationPlaceholder")}
                  value={workspaceCanonicalUri}
                  onChange={handleWorkspaceCanonicalUriChange}
                />
              </div>
            </div>
          )}
        </div>
        <div className="dialog-footer">
          <Button type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
          <Button type="submit">{submitLabel}</Button>
        </div>
      </form>
    </DialogShell>
  );
}
