import { Network, Pencil, Power, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { StorageRoot, TeamLibrary } from "../../api";
import type { TranslatorContext } from "../../types";
import { storageKindLabel } from "../../utils/format";
import { Panel, SelectField, StatusDot } from "../common";

export function StorageRootsPanel({
  libraries,
  selectedLibraryId,
  storageRoots,
  t,
  onDelete,
  onEdit,
  onLibraryChange,
  onToggle
}: TranslatorContext & {
  libraries: TeamLibrary[];
  selectedLibraryId: string;
  storageRoots: StorageRoot[];
  onDelete: (root: StorageRoot) => void;
  onEdit: (root: StorageRoot) => void;
  onLibraryChange: (libraryId: string) => void;
  onToggle: (root: StorageRoot) => void;
}) {
  return (
    <Panel
      title={t("sharedRoots")}
      icon={Network}
      className="storage-roots-card"
    >
      <div className="toolbar-strip">
        <SelectField className="compact-select-field" label={t("selectLibrary")} value={selectedLibraryId} onChange={onLibraryChange}>
            {libraries.map((item) => (
              <option key={item.id} value={item.id}>{item.displayName}</option>
            ))}
        </SelectField>
        <StatusDot label={storageRoots.length ? t("realData") : t("empty")} tone={storageRoots.length ? "good" : "muted"} />
      </div>
      <div className="table-wrap">
        <table className="data-table">
          <thead>
            <tr>
              <th>{t("name")}</th>
              <th>{t("kind")}</th>
              <th>{t("workspaceLocation")}</th>
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
                  <td>{storageKindLabel(t, item.kind)}</td>
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
