import type { FormEvent } from "react";
import { Button } from "@/components/ui/button";
import type { StorageRoot, TeamLibrary } from "../../api";
import type { TranslatorContext } from "../../types";
import { TextField } from "../common";
import { DialogShell } from "./dialog-shell";

export function StorageRootDialog({
  canonicalUri,
  editingRoot,
  enabled,
  kind,
  libraries,
  macosAliases,
  macosSmbUrl,
  name,
  open,
  selectedLibraryId,
  t,
  windowsAliases,
  windowsUncPath,
  onCanonicalUriChange,
  onClose,
  onEnabledChange,
  onKindChange,
  onLibraryChange,
  onMacosAliasesChange,
  onMacosSmbUrlChange,
  onNameChange,
  onSubmit,
  onWindowsAliasesChange,
  onWindowsUncPathChange
}: TranslatorContext & {
  canonicalUri: string;
  editingRoot: StorageRoot | null;
  enabled: boolean;
  kind: string;
  libraries: TeamLibrary[];
  macosAliases: string;
  macosSmbUrl: string;
  name: string;
  open: boolean;
  selectedLibraryId: string;
  windowsAliases: string;
  windowsUncPath: string;
  onCanonicalUriChange: (value: string) => void;
  onClose: () => void;
  onEnabledChange: (value: boolean) => void;
  onKindChange: (value: string) => void;
  onLibraryChange: (value: string) => void;
  onMacosAliasesChange: (value: string) => void;
  onMacosSmbUrlChange: (value: string) => void;
  onNameChange: (value: string) => void;
  onSubmit: (event: FormEvent) => void | Promise<void>;
  onWindowsAliasesChange: (value: string) => void;
  onWindowsUncPathChange: (value: string) => void;
}) {
  return (
    <DialogShell
      className="storage-dialog"
      closeLabel={t("cancel")}
      open={open}
      subtitle={t("storagePageHint")}
      title={editingRoot ? t("editStorageRoot") : t("createStorageRoot")}
      titleId="storage-dialog-title"
      onClose={onClose}
    >
      <form className="dialog-form" onSubmit={onSubmit}>
        <div className="dialog-body dialog-grid">
          <label className="field">
            <span>{t("selectLibrary")}</span>
            <select value={selectedLibraryId} onChange={(event) => onLibraryChange(event.target.value)} disabled={Boolean(editingRoot)}>
              {libraries.map((item) => (
                <option key={item.id} value={item.id}>{item.displayName}</option>
              ))}
            </select>
          </label>
          <TextField autoFocus required label={t("name")} value={name} onChange={onNameChange} />
          <label className="field">
            <span>{t("kind")}</span>
            <select value={kind} onChange={(event) => onKindChange(event.target.value)}>
              <option value="server_filesystem">{t("storageKindServerFilesystem")}</option>
              <option value="smb">{t("storageKindSmb")}</option>
              <option value="s3">{t("storageKindS3")}</option>
            </select>
          </label>
          <label className="field">
            <span>{t("status")}</span>
            <select value={enabled ? "enabled" : "disabled"} onChange={(event) => onEnabledChange(event.target.value === "enabled")}>
              <option value="enabled">{t("enabled")}</option>
              <option value="disabled">{t("disabled")}</option>
            </select>
          </label>
          <div className="span-field">
            <TextField required label={t("workspaceLocation")} value={canonicalUri} onChange={onCanonicalUriChange} />
          </div>
        </div>
        <div className="dialog-footer">
          <Button type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
          <Button type="submit">{t("submit")}</Button>
        </div>
      </form>
    </DialogShell>
  );
}
