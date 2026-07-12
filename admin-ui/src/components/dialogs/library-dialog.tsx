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
  name,
  open,
  showStorage = false,
  storageLocked = false,
  storageConnectionId = "",
  storageConnections = [],
  submitLabel,
  t,
  title,
  onClose,
  onNameChange,
  onStorageConnectionChange,
  onCreateStorage,
  onSubmit
}: TranslatorContext & {
  hint: string;
  name: string;
  open: boolean;
  showStorage?: boolean;
  storageLocked?: boolean;
  storageConnectionId?: string;
  storageConnections?: StorageConnection[];
  submitLabel: string;
  title: string;
  onClose: () => void;
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
          <TextField autoFocus required label={t("name")} value={name} onChange={onNameChange} />
          {showStorage && onStorageConnectionChange && (
            availableStorageConnections.length > 0 ? (
              <SelectField
                disabled={storageLocked}
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
          {showStorage && storageLocked && <p className="dialog-hint">{t("libraryStorageMigrationRequired")}</p>}
        </div>
        <div className="dialog-footer">
          <Button type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
          <Button disabled={showStorage && !storageConnectionId} type="submit">{submitLabel}</Button>
        </div>
      </form>
    </DialogShell>
  );
}
