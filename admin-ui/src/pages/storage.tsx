import { useState } from "react";
import { HardDrive } from "lucide-react";
import type { StorageRoot } from "../api";
import { api } from "../api";
import type { PageContext } from "../types";
import { StorageRootDialog } from "../components/dialogs";
import { InfoStack, Panel } from "../components/common";
import { StorageRootsPanel } from "../components/storage/storage-roots-panel";
import { splitList } from "../utils/format";
import { storageRootPayloadFromRecord } from "../utils/storage-roots";

export function StoragePage({ t, token, libraries, selectedLibraryId, setSelectedLibraryId, storageRoots, deploymentMode, refreshAll, setMessage }: PageContext) {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingRoot, setEditingRoot] = useState<StorageRoot | null>(null);
  const [name, setName] = useState("");
  const [kind, setKind] = useState("smb");
  const [canonicalUri, setCanonicalUri] = useState("");
  const [windowsUncPath, setWindowsUncPath] = useState("");
  const [windowsAliases, setWindowsAliases] = useState("");
  const [macosSmbUrl, setMacosSmbUrl] = useState("");
  const [macosAliases, setMacosAliases] = useState("");
  const [rootEnabled, setRootEnabled] = useState(true);

  const resetRootForm = () => {
    setName("");
    setKind("smb");
    setCanonicalUri("");
    setWindowsUncPath("");
    setWindowsAliases("");
    setMacosSmbUrl("");
    setMacosAliases("");
    setRootEnabled(true);
  };

  const openCreateDialog = () => {
    setEditingRoot(null);
    resetRootForm();
    setDialogOpen(true);
  };

  const openEditDialog = (root: StorageRoot) => {
    setEditingRoot(root);
    setSelectedLibraryId(root.libraryId);
    setName(root.name);
    setKind(root.kind);
    setCanonicalUri(root.canonicalUri);
    setWindowsUncPath(root.windowsUncPath ?? "");
    setWindowsAliases(root.windowsMappedDriveAliases.join(", "));
    setMacosSmbUrl(root.macosSmbUrl ?? "");
    setMacosAliases(root.macosMountAliases.join(", "));
    setRootEnabled(root.enabled);
    setDialogOpen(true);
  };

  const closeDialog = () => {
    setDialogOpen(false);
  };

  const submitRoot = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!selectedLibraryId || !name.trim() || !canonicalUri.trim()) {
      setMessage(t("formRequiredHint"));
      return;
    }
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      const payload = {
        name: name.trim(),
        kind,
        canonicalUri: canonicalUri.trim(),
        windowsUncPath: windowsUncPath.trim() || undefined,
        windowsMappedDriveAliases: splitList(windowsAliases),
        macosSmbUrl: macosSmbUrl.trim() || undefined,
        macosMountAliases: splitList(macosAliases),
        enabled: rootEnabled
      };
      if (editingRoot) {
        await api.updateStorageRoot(token, editingRoot.id, payload);
      } else {
        await api.createStorageRoot(token, {
          libraryId: selectedLibraryId,
          ...payload
        });
      }
      closeDialog();
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const toggleStorageRoot = async (root: StorageRoot) => {
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.updateStorageRoot(token, root.id, storageRootPayloadFromRecord(root, !root.enabled));
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const deleteStorageRoot = async (root: StorageRoot) => {
    if (!window.confirm(t("deleteStorageRootConfirm"))) return;
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.deleteStorageRoot(token, root.id);
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <div className="page-grid">
      <StorageRootDialog
        canonicalUri={canonicalUri}
        enabled={rootEnabled}
        editingRoot={editingRoot}
        kind={kind}
        libraries={libraries}
        macosAliases={macosAliases}
        macosSmbUrl={macosSmbUrl}
        name={name}
        open={dialogOpen}
        selectedLibraryId={selectedLibraryId}
        t={t}
        windowsAliases={windowsAliases}
        windowsUncPath={windowsUncPath}
        onCanonicalUriChange={setCanonicalUri}
        onClose={closeDialog}
        onEnabledChange={setRootEnabled}
        onKindChange={setKind}
        onLibraryChange={setSelectedLibraryId}
        onMacosAliasesChange={setMacosAliases}
        onMacosSmbUrlChange={setMacosSmbUrl}
        onNameChange={setName}
        onSubmit={submitRoot}
        onWindowsAliasesChange={setWindowsAliases}
        onWindowsUncPathChange={setWindowsUncPath}
      />
      <Panel title={t("storage")} icon={HardDrive} className="span-4">
        <InfoStack
          items={[
            [t("provider"), deploymentMode === "local" ? "Filesystem" : "S3 / MinIO"],
            [t("objectStorage"), deploymentMode === "cloud" ? t("notConfigured") : t("disabled")],
            [t("pathResolver"), t("enabled")]
          ]}
        />
      </Panel>
      <StorageRootsPanel
        libraries={libraries}
        selectedLibraryId={selectedLibraryId}
        storageRoots={storageRoots}
        t={t}
        onCreate={openCreateDialog}
        onDelete={(root) => void deleteStorageRoot(root)}
        onEdit={openEditDialog}
        onLibraryChange={setSelectedLibraryId}
        onToggle={(root) => void toggleStorageRoot(root)}
      />
    </div>
  );
}
