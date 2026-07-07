import { Network, Pencil, Plus, Power, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { StorageRoot, TeamLibrary } from "../../api";
import type { TranslatorContext } from "../../types";
import { Panel, StatusDot } from "../common";

export function StorageRootsPanel({
  libraries,
  selectedLibraryId,
  storageRoots,
  t,
  onCreate,
  onDelete,
  onEdit,
  onLibraryChange,
  onToggle
}: TranslatorContext & {
  libraries: TeamLibrary[];
  selectedLibraryId: string;
  storageRoots: StorageRoot[];
  onCreate: () => void;
  onDelete: (root: StorageRoot) => void;
  onEdit: (root: StorageRoot) => void;
  onLibraryChange: (libraryId: string) => void;
  onToggle: (root: StorageRoot) => void;
}) {
  return (
    <Panel
      title={t("sharedRoots")}
      icon={Network}
      className="span-8"
      action={
        <Button className="panel-action-button" size="sm" type="button" onClick={onCreate} disabled={libraries.length === 0}>
          <Plus size={15} />
          <span>{t("createStorageRoot")}</span>
        </Button>
      }
    >
      <div className="toolbar-strip">
        <label className="field compact-select-field">
          <span>{t("selectLibrary")}</span>
          <select value={selectedLibraryId} onChange={(event) => onLibraryChange(event.target.value)}>
            {libraries.map((item) => (
              <option key={item.id} value={item.id}>{item.displayName}</option>
            ))}
          </select>
        </label>
        <StatusDot label={storageRoots.length ? t("realData") : t("empty")} tone={storageRoots.length ? "good" : "muted"} />
      </div>
      <div className="table-wrap">
        <table className="data-table">
          <thead>
            <tr>
              <th>{t("name")}</th>
              <th>{t("provider")}</th>
              <th>{t("canonicalUri")}</th>
              <th>{t("status")}</th>
              <th>{t("action")}</th>
            </tr>
          </thead>
          <tbody>
            {storageRoots.length === 0 ? (
              <tr>
                <td colSpan={5}>{t("noSharedRoots")}</td>
              </tr>
            ) : (
              storageRoots.map((item) => (
                <tr key={item.id}>
                  <td>{item.name}</td>
                  <td>{item.kind}</td>
                  <td>{item.canonicalUri}</td>
                  <td><span className={`status-pill${item.enabled ? " is-on" : " is-off"}`}>{item.enabled ? t("enabled") : t("disabled")}</span></td>
                  <td>
                    <div className="table-actions">
                      <Button className="table-action-button" size="sm" type="button" variant="outline" onClick={() => onEdit(item)}>
                        <Pencil size={14} />
                        <span>{t("edit")}</span>
                      </Button>
                      <Button className="table-action-button" size="sm" type="button" variant="outline" onClick={() => onToggle(item)}>
                        <Power size={14} />
                        <span>{item.enabled ? t("deactivate") : t("activate")}</span>
                      </Button>
                      <Button className="table-action-button is-danger" size="sm" type="button" variant="outline" onClick={() => onDelete(item)}>
                        <Trash2 size={14} />
                        <span>{t("delete")}</span>
                      </Button>
                    </div>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </Panel>
  );
}
