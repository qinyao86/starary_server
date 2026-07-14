import { useState } from "react";
import type { FormEvent } from "react";
import { ArrowRightLeft, Copy, Plus, Star, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import emptyStorageIllustration from "../assets/empty-storage.svg";
import type { StorageConnection } from "../api";
import { api } from "../api";
import { EmptyState, PageFrame } from "../components/common";
import { StorageConnectionDialog, StorageMigrationDialog } from "../components/dialogs";
import type { PageContext } from "../types";
import { storageKindLabel } from "../utils/format";

export function StoragePage({ t, token, currentUser, storageConnections, refreshAll, setMessage }: PageContext) {
  const [open, setOpen] = useState(false);
  const [kind, setKind] = useState("server_filesystem");
  const [location, setLocation] = useState("");
  const [migrationOpen, setMigrationOpen] = useState(false);
  const [migrationTarget, setMigrationTarget] = useState<StorageConnection | null>(null);
  const [migrationKind, setMigrationKind] = useState("server_filesystem");
  const [migrationLocation, setMigrationLocation] = useState("");
  const [migrating, setMigrating] = useState(false);
  const canManageStorage = currentUser?.role === "owner" || currentUser?.role === "admin";

  const openCreate = () => {
    setKind("server_filesystem");
    setLocation("");
    setOpen(true);
  };

  const openMigration = (connection: StorageConnection) => {
    setMigrationTarget(connection);
    setMigrationKind(connection.kind === "s3" ? "server_filesystem" : connection.kind);
    setMigrationLocation("");
    setMigrationOpen(true);
  };

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (!location.trim()) {
      setMessage(t("formRequiredHint"));
      return;
    }
    try {
      const payload = {
        kind,
        canonicalUri: location.trim(),
        windowsMappedDriveAliases: [],
        macosMountAliases: []
      };
      await api.createStorageConnection(token, payload);
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

  const migrate = async (event: FormEvent) => {
    event.preventDefault();
    if (!migrationTarget || !migrationLocation.trim() || migrating) return;
    setMigrating(true);
    try {
      await api.migrateStorageConnection(token, migrationTarget.id, {
        kind: migrationKind,
        canonicalUri: migrationLocation.trim(),
        windowsMappedDriveAliases: [],
        macosMountAliases: []
      });
      setMigrationOpen(false);
      setMigrationTarget(null);
      setMessage(t("storageMigrationCompleted"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setMigrating(false);
    }
  };

  const setDefault = async (connection: StorageConnection) => {
    try {
      await api.setDefaultStorageConnection(token, connection.id);
      setMessage(t("defaultStorageUpdated"));
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
      className={storageConnections.length === 0 ? "management-empty-page-frame" : ""}
      title={t("storage")}
      description={t("storageConnectionsPageHint")}
      action={canManageStorage ? <Button type="button" onClick={openCreate}><Plus size={16} /><span>{t("createStorageConnection")}</span></Button> : undefined}
    >
      <StorageConnectionDialog
        connection={null}
        kind={kind}
        location={location}
        open={open}
        t={t}
        onClose={() => setOpen(false)}
        onKindChange={setKind}
        onLocationChange={setLocation}
        onSubmit={submit}
      />
      <StorageMigrationDialog
        busy={migrating}
        connection={migrationTarget}
        kind={migrationKind}
        location={migrationLocation}
        open={migrationOpen}
        t={t}
        onClose={() => {
          if (!migrating) {
            setMigrationOpen(false);
            setMigrationTarget(null);
          }
        }}
        onKindChange={setMigrationKind}
        onLocationChange={setMigrationLocation}
        onSubmit={migrate}
      />
      {storageConnections.length === 0 ? (
        <EmptyState illustration={emptyStorageIllustration} label={t("noStorageConnections")} />
      ) : <div className="table-wrap management-list-card storage-connections-table">
        <table className="data-table">
          <thead><tr><th className="storage-path-column">{t("libraryStorageLocation")}</th><th className="storage-kind-column">{t("kind")}</th><th className="storage-count-column">{t("linkedLibraries")}</th>{canManageStorage && <th className="storage-actions-column">{t("action")}</th>}</tr></thead>
          <tbody>
            {storageConnections.map((connection) => (
              <tr key={connection.id}>
                <td className="storage-path-column storage-connection-path">
                  <div className="storage-connection-path-line">
                    <strong title={connection.canonicalUri}>{connection.canonicalUri}</strong>
                    {connection.isDefault ? <Badge className="storage-default-badge" variant="secondary">{t("defaultStorage")}</Badge> : null}
                  </div>
                </td>
                <td className="storage-kind-column">{storageKindLabel(t, connection.kind)}</td>
                <td className="storage-count-column">{connection.libraryCount}</td>
                {canManageStorage && <td className="storage-actions-column"><div className="storage-connection-actions">
                  {!connection.isDefault ? <Button className="storage-action-button" size="icon" type="button" variant="ghost" title={t("setAsDefaultStorage")} aria-label={t("setAsDefaultStorage")} disabled={!connection.enabled} onClick={() => void setDefault(connection)}><Star size={14} /></Button> : null}
                  <Button className="storage-action-button" size="icon" type="button" variant="ghost" title={t("copyStoragePath")} aria-label={t("copyStoragePath")} onClick={() => void copyPath(connection.canonicalUri)}><Copy size={14} /></Button>
                  <Button className="storage-action-button" size="icon" type="button" variant="ghost" title={connection.kind === "s3" ? t("objectStorageMigrationUnavailable") : t("migrateStorageConnection")} aria-label={t("migrateStorageConnection")} disabled={connection.kind === "s3"} onClick={() => openMigration(connection)}><ArrowRightLeft size={15} /></Button>
                  <Button className="storage-action-button is-danger" size="icon" type="button" variant="ghost" title={t("delete")} aria-label={t("delete")} disabled={connection.libraryCount > 0} onClick={() => void remove(connection)}><Trash2 size={15} /></Button>
                </div></td>}
              </tr>
            ))}
          </tbody>
        </table>
      </div>}
    </PageFrame>
  );
}
