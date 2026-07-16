import type { FormEvent } from "react";
import { Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { StorageConnection } from "../../api";
import type { TranslatorContext } from "../../types";
import { SelectField, TextField } from "../common";
import { DialogShell } from "./dialog-shell";

const createStorageValue = "__create_storage__";

export function LibraryDialog({
  hint,
  accessMode = "invite",
  name,
  open,
  showAccessMode = true,
  showName = true,
  showStorage = false,
  storageConnectionId = "",
  storageConnections = [],
  submitLabel,
  t,
  title,
  onClose,
  onAccessModeChange,
  onNameChange,
  onStorageConnectionChange,
  onCreateStorage,
  onSubmit
}: TranslatorContext & {
  hint: string;
  accessMode?: "public" | "invite";
  name: string;
  open: boolean;
  showAccessMode?: boolean;
  showName?: boolean;
  showStorage?: boolean;
  storageConnectionId?: string;
  storageConnections?: StorageConnection[];
  submitLabel: string;
  title: string;
  onClose: () => void;
  onAccessModeChange?: (value: "public" | "invite") => void;
  onNameChange: (value: string) => void;
  onStorageConnectionChange?: (value: string) => void;
  onCreateStorage?: () => void;
  onSubmit: (event: FormEvent) => void | Promise<void>;
}) {
  const availableStorageConnections = storageConnections.filter(
    (connection) => connection.enabled || connection.id === storageConnectionId
  );

  return (
    <DialogShell className="library-dialog" closeLabel={t("cancel")} open={open} subtitle={hint} title={title} titleId="library-dialog-title" onClose={onClose}>
      <form className="dialog-form" onSubmit={onSubmit}>
        <div className="dialog-body">
          {showName && <TextField autoFocus required label={t("name")} value={name} onChange={onNameChange} />}
          {showAccessMode && onAccessModeChange && (
            <SelectField
              label={t("libraryAccessMode")}
              value={accessMode}
              onChange={(value) => onAccessModeChange(value === "public" ? "public" : "invite")}
            >
              <option value="invite">{t("libraryAccessInvite")}</option>
              <option value="public">{t("libraryAccessPublic")}</option>
            </SelectField>
          )}
          {showStorage && onStorageConnectionChange && (
            availableStorageConnections.length > 0 ? (
              <SelectField
                required
                label={t("libraryStorageLocation")}
                value={storageConnectionId || availableStorageConnections[0].id}
                onChange={(value) => {
                  if (value === createStorageValue) {
                    onCreateStorage?.();
                    return;
                  }
                  onStorageConnectionChange(value);
                }}
              >
                {availableStorageConnections.map((connection) => (
                  <option key={connection.id} value={connection.id}>{connection.canonicalUri}</option>
                ))}
                {onCreateStorage && <option value={createStorageValue}>{t("createStorageLocationOption")}</option>}
              </SelectField>
            ) : (
              <label className="field">
                <span>{t("libraryStorageLocation")}</span>
                <button className="select-control-button" type="button" onClick={onCreateStorage}>
                  <span>{t("createStorageLocationOption")}</span>
                  <Plus aria-hidden="true" size={15} />
                </button>
              </label>
            )
          )}
        </div>
        <div className="dialog-footer">
          <Button type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
          <Button disabled={showStorage && !storageConnectionId} type="submit">{submitLabel}</Button>
        </div>
      </form>
    </DialogShell>
  );
}
