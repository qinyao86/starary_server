import { useState } from "react";
import type { FormEvent } from "react";
import { Copy, Pencil, Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { StorageConnection } from "../api";
import { api } from "../api";
import { PageFrame } from "../components/common";
import { StorageConnectionDialog } from "../components/dialogs";
import type { PageContext } from "../types";
import { storageKindLabel } from "../utils/format";

export function StoragePage({ t, token, currentUser, storageConnections, refreshAll, setMessage }: PageContext) {
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<StorageConnection | null>(null);
  const [kind, setKind] = useState("server_filesystem");
  const [location, setLocation] = useState("");
  const canManageStorage = currentUser?.role === "owner" || currentUser?.role === "admin";

  const openCreate = () => {
    setEditing(null);
    setKind("server_filesystem");
    setLocation("");
    setOpen(true);
  };

  const openEdit = (connection: StorageConnection) => {
    setEditing(connection);
    setKind(connection.kind);
    setLocation(connection.canonicalUri);
    setOpen(true);
  };

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (!location.trim()) {
      setMessage(t("formRequiredHint"));
      return;
    }
    try {
      const preservePlatformMappings = editing?.kind === kind;
      const payload = {
        kind,
        canonicalUri: location.trim(),
        windowsUncPath: preservePlatformMappings ? editing?.windowsUncPath ?? undefined : undefined,
        windowsMappedDriveAliases: preservePlatformMappings ? editing?.windowsMappedDriveAliases ?? [] : [],
        macosSmbUrl: preservePlatformMappings ? editing?.macosSmbUrl ?? undefined : undefined,
        macosMountAliases: preservePlatformMappings ? editing?.macosMountAliases ?? [] : []
      };
      if (editing) await api.updateStorageConnection(token, editing.id, { ...payload, enabled: editing.enabled });
      else await api.createStorageConnection(token, payload);
      setOpen(false);
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const remove = async (connection: StorageConnection) => {
    if (!window.confirm(t("deleteStorageConnectionConfirm"))) return;
    try {
      await api.deleteStorageConnection(token, connection.id);
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const copyPath = async (path: string) => {
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(path);
      } else {
        const textarea = document.createElement("textarea");
        textarea.value = path;
        textarea.setAttribute("readonly", "true");
        textarea.style.position = "fixed";
        textarea.style.left = "-9999px";
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand("copy");
        document.body.removeChild(textarea);
      }
      setMessage(t("storagePathCopied"));
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <PageFrame
      title={t("storage")}
      description={t("storageConnectionsPageHint")}
      action={canManageStorage ? <Button type="button" onClick={openCreate}><Plus size={16} /><span>{t("createStorageConnection")}</span></Button> : undefined}
    >
      <StorageConnectionDialog
        connection={editing}
        kind={kind}
        location={location}
        open={open}
        t={t}
        onClose={() => setOpen(false)}
        onKindChange={setKind}
        onLocationChange={setLocation}
        onSubmit={submit}
      />
      <div className="table-wrap storage-connections-table">
        <table className="data-table">
          <thead><tr><th className="storage-path-column">{t("libraryStorageLocation")}</th><th className="storage-kind-column">{t("kind")}</th><th className="storage-count-column">{t("linkedLibraries")}</th>{canManageStorage && <th className="storage-actions-column">{t("action")}</th>}</tr></thead>
          <tbody>
            {storageConnections.length === 0 ? <tr><td colSpan={canManageStorage ? 4 : 3}>{t("noStorageConnections")}</td></tr> : storageConnections.map((connection) => (
              <tr key={connection.id}>
                <td className="storage-path-column storage-connection-path"><strong title={connection.canonicalUri}>{connection.canonicalUri}</strong></td>
                <td className="storage-kind-column">{storageKindLabel(t, connection.kind)}</td>
                <td className="storage-count-column">{connection.libraryCount}</td>
                {canManageStorage && <td className="storage-actions-column"><div className="storage-connection-actions">
                  <Button className="storage-action-button" size="icon" type="button" variant="ghost" title={t("copyStoragePath")} aria-label={t("copyStoragePath")} onClick={() => void copyPath(connection.canonicalUri)}><Copy size={14} /></Button>
                  <Button className="storage-action-button" size="icon" type="button" variant="ghost" title={t("edit")} aria-label={t("edit")} onClick={() => openEdit(connection)}><Pencil size={15} /></Button>
                  <Button className="storage-action-button is-danger" size="icon" type="button" variant="ghost" title={t("delete")} aria-label={t("delete")} disabled={connection.libraryCount > 0} onClick={() => void remove(connection)}><Trash2 size={15} /></Button>
                </div></td>}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </PageFrame>
  );
}
