import { useState } from "react";
import type { FormEvent } from "react";
import { Pencil, Plus, Power, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { StorageConnection } from "../api";
import { api } from "../api";
import { PageFrame, StatusDot } from "../components/common";
import { StorageConnectionDialog } from "../components/dialogs";
import type { PageContext } from "../types";
import { storageKindLabel } from "../utils/format";

export function StoragePage({ t, token, currentUser, storageConnections, refreshAll, setMessage }: PageContext) {
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<StorageConnection | null>(null);
  const [kind, setKind] = useState("server_filesystem");
  const [location, setLocation] = useState("");
  const [enabled, setEnabled] = useState(true);
  const canManageStorage = currentUser?.role === "owner" || currentUser?.role === "admin";

  const openCreate = () => {
    setEditing(null);
    setKind("server_filesystem");
    setLocation("");
    setEnabled(true);
    setOpen(true);
  };

  const openEdit = (connection: StorageConnection) => {
    setEditing(connection);
    setKind(connection.kind);
    setLocation(connection.canonicalUri);
    setEnabled(connection.enabled);
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
      if (editing) await api.updateStorageConnection(token, editing.id, { ...payload, enabled });
      else await api.createStorageConnection(token, payload);
      setOpen(false);
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const toggle = async (connection: StorageConnection) => {
    try {
      await api.updateStorageConnection(token, connection.id, {
        kind: connection.kind,
        canonicalUri: connection.canonicalUri,
        windowsUncPath: connection.windowsUncPath ?? undefined,
        windowsMappedDriveAliases: connection.windowsMappedDriveAliases,
        macosSmbUrl: connection.macosSmbUrl ?? undefined,
        macosMountAliases: connection.macosMountAliases,
        enabled: !connection.enabled
      });
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

  return (
    <PageFrame
      title={t("storage")}
      description={t("storageConnectionsPageHint")}
      action={canManageStorage ? <Button type="button" onClick={openCreate}><Plus size={16} /><span>{t("createStorageConnection")}</span></Button> : undefined}
    >
      <StorageConnectionDialog
        connection={editing}
        enabled={enabled}
        kind={kind}
        location={location}
        open={open}
        t={t}
        onClose={() => setOpen(false)}
        onEnabledChange={setEnabled}
        onKindChange={setKind}
        onLocationChange={setLocation}
        onSubmit={submit}
      />
      <div className="table-wrap storage-connections-table">
        <table className="data-table">
          <thead><tr><th>{t("libraryStorageLocation")}</th><th>{t("kind")}</th><th>{t("linkedLibraries")}</th><th>{t("status")}</th>{canManageStorage && <th>{t("action")}</th>}</tr></thead>
          <tbody>
            {storageConnections.length === 0 ? <tr><td colSpan={canManageStorage ? 5 : 4}>{t("noStorageConnections")}</td></tr> : storageConnections.map((connection) => (
              <tr key={connection.id}>
                <td className="storage-connection-path"><strong>{connection.canonicalUri}</strong></td>
                <td>{storageKindLabel(t, connection.kind)}</td>
                <td>{connection.libraryCount}</td>
                <td><StatusDot label={connection.enabled ? t("enabled") : t("disabled")} tone={connection.enabled ? "good" : "muted"} /></td>
                {canManageStorage && <td><div className="table-actions">
                  <Button size="icon" type="button" variant="outline" title={t("edit")} onClick={() => openEdit(connection)}><Pencil size={14} /></Button>
                  <Button size="icon" type="button" variant="outline" title={connection.enabled ? t("deactivate") : t("activate")} onClick={() => void toggle(connection)}><Power size={14} /></Button>
                  <Button size="icon" type="button" variant="outline" title={t("delete")} disabled={connection.libraryCount > 0} onClick={() => void remove(connection)}><Trash2 size={14} /></Button>
                </div></td>}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </PageFrame>
  );
}
