import { useState } from "react";
import type { FormEvent } from "react";
import type { LibraryMember, TeamLibrary } from "../api";
import { api } from "../api";
import { ApiError } from "../api/request";
import { StorageConnectionDialog } from "../components/dialogs";
import { LibraryDetailPageView, LibraryListPageView } from "../components/libraries/library-page-views";
import { useAvailableLibraryUsers } from "../hooks/use-available-library-users";
import type { PageContext } from "../types";

export function LibrariesPage({
  t, token, currentUser, libraries, users, storageRoots, storageConnections, libraryMembers, libraryActivityItems, selectedLibraryId, setSelectedLibraryId, refreshAll, setMessage
}: PageContext) {
  const [viewLibraryId, setViewLibraryId] = useState<string | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [editingLibraryId, setEditingLibraryId] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [storageConnectionId, setStorageConnectionId] = useState("");
  const [storageDialogOpen, setStorageDialogOpen] = useState(false);
  const [storageKind, setStorageKind] = useState("server_filesystem");
  const [storageLocation, setStorageLocation] = useState("");
  const [editName, setEditName] = useState("");
  const [editDescription, setEditDescription] = useState("");
  const [memberUserId, setMemberUserId] = useState("");
  const [memberRole, setMemberRole] = useState("viewer");
  const [memberDialogOpen, setMemberDialogOpen] = useState(false);
  const activeLibrary = libraries.find((item) => item.id === viewLibraryId) ?? null;
  const availableUsers = useAvailableLibraryUsers(users, libraryMembers, memberUserId, setMemberUserId);
  const canCreateLibrary = currentUser?.role === "owner" || currentUser?.role === "admin";
  const canManageLibrary = (library: TeamLibrary) =>
    ["owner", "admin", "library_manager"].includes(library.currentUserRole ?? currentUser?.role ?? "");
  const storageErrorMessage = (error: unknown) => {
    if (error instanceof ApiError && error.code === "storage_location_conflict") {
      return t("storageLocationConflict");
    }
    if (error instanceof ApiError && error.code === "storage_migration_required") {
      return t("libraryStorageMigrationRequired");
    }
    return error instanceof Error ? error.message : String(error);
  };

  const openCreateLibraryDialog = () => {
    cancelEditLibrary();
    setName("");
    setDescription("");
    setStorageConnectionId(storageConnections.find((connection) => connection.enabled)?.id ?? "");
    setShowCreate(true);
  };

  const closeCreateLibraryDialog = () => {
    setShowCreate(false);
    setName("");
    setDescription("");
    setStorageConnectionId("");
  };

  const openLibrary = (libraryId: string) => {
    setViewLibraryId(libraryId);
    setSelectedLibraryId(libraryId);
  };

  const startEditLibrary = (library: TeamLibrary) => {
    setShowCreate(false);
    setEditingLibraryId(library.id);
    setEditName(library.displayName);
    setEditDescription(library.description ?? "");
    setStorageConnectionId(library.primaryStorageConnectionId ?? storageConnections.find((connection) => connection.enabled)?.id ?? "");
  };

  const cancelEditLibrary = () => {
    setEditingLibraryId(null);
    setEditName("");
    setEditDescription("");
    setStorageConnectionId("");
  };

  const openStorageDialog = () => {
    setStorageKind("server_filesystem");
    setStorageLocation("");
    setStorageDialogOpen(true);
  };

  const createStorageConnection = async (event: FormEvent) => {
    event.preventDefault();
    if (!storageLocation.trim()) {
      setMessage(t("formRequiredHint"));
      return;
    }
    try {
      const connection = await api.createStorageConnection(token, {
        kind: storageKind,
        canonicalUri: storageLocation.trim(),
        windowsMappedDriveAliases: [],
        macosMountAliases: []
      });
      setStorageConnectionId(connection.id);
      setStorageDialogOpen(false);
      await refreshAll();
    } catch (error) {
      setMessage(storageErrorMessage(error));
    }
  };

  const openMemberDialog = () => {
    setMemberUserId(availableUsers[0]?.id ?? "");
    setMemberRole("viewer");
    setMemberDialogOpen(true);
  };

  const createLibrary = async (event: FormEvent) => {
    event.preventDefault();
    if (!name.trim() || !storageConnectionId) {
      setMessage(t("formRequiredHint"));
      return;
    }
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.createLibrary(token, {
        displayName: name.trim(),
        description: description.trim() || undefined,
        storageBinding: { connectionId: storageConnectionId }
      });
      closeCreateLibraryDialog();
      setMessage(t("saved"));
      await refreshAll();
      setViewLibraryId(null);
    } catch (error) {
      setMessage(storageErrorMessage(error));
    }
  };

  const updateLibrary = async (event: FormEvent) => {
    event.preventDefault();
    if (!editingLibraryId || !editName.trim() || !storageConnectionId) {
      setMessage(t("formRequiredHint"));
      return;
    }
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.updateLibrary(token, editingLibraryId, {
        displayName: editName.trim(),
        description: editDescription.trim() || undefined,
        storageBinding: { connectionId: storageConnectionId }
      });
      cancelEditLibrary();
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(storageErrorMessage(error));
    }
  };

  const toggleLibraryEnabled = async (library: TeamLibrary, enabled: boolean) => {
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.updateLibraryEnabled(token, library.id, { enabled });
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const deleteLibrary = async (library: TeamLibrary) => {
    if (!window.confirm(t("deleteLibraryConfirm"))) return;
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.deleteLibrary(token, library.id);
      if (viewLibraryId === library.id) setViewLibraryId(null);
      if (selectedLibraryId === library.id) setSelectedLibraryId("");
      if (editingLibraryId === library.id) cancelEditLibrary();
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const submitMember = async (event: FormEvent) => {
    event.preventDefault();
    if (!selectedLibraryId || !memberUserId) {
      setMessage(t("formRequiredHint"));
      return;
    }
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.upsertLibraryMember(token, selectedLibraryId, memberUserId, { role: memberRole });
      setMemberDialogOpen(false);
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const removeMember = async (member: LibraryMember) => {
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.removeLibraryMember(token, member.libraryId, member.userId);
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  if (activeLibrary) {
    return (
      <LibraryDetailPageView
        activeLibrary={activeLibrary}
        availableUsers={availableUsers}
        libraryActivityItems={libraryActivityItems}
        libraryMembers={libraryMembers}
        memberDialogOpen={memberDialogOpen}
        memberRole={memberRole}
        memberUserId={memberUserId}
        storageRoots={storageRoots}
        t={t}
        onBack={() => setViewLibraryId(null)}
        onMemberDialogClose={() => setMemberDialogOpen(false)}
        onMemberDialogOpen={openMemberDialog}
        onMemberRoleChange={setMemberRole}
        onMemberSubmit={submitMember}
        onMemberUserChange={setMemberUserId}
        onRemoveMember={(member) => void removeMember(member)}
      />
    );
  }

  return (
    <>
      <StorageConnectionDialog
        connection={null}
        kind={storageKind}
        location={storageLocation}
        open={storageDialogOpen}
        subtitle={t("createStorageLocationHint")}
        t={t}
        title={t("createStorageLocation")}
        onClose={() => setStorageDialogOpen(false)}
        onKindChange={setStorageKind}
        onLocationChange={setStorageLocation}
        onSubmit={createStorageConnection}
      />
      <LibraryListPageView
      createDescription={description}
      createName={name}
      editDescription={editDescription}
      editName={editName}
      editingLibraryId={storageDialogOpen ? null : editingLibraryId}
      editingStorageLocked={Boolean(libraries.find((library) => library.id === editingLibraryId)?.storageLockedAt)}
      libraries={libraries}
      canCreateLibrary={canCreateLibrary}
      canManageLibrary={canManageLibrary}
      showCreate={showCreate && !storageDialogOpen}
      t={t}
      storageConnectionId={storageConnectionId}
      storageConnections={storageConnections}
      onCancelEdit={cancelEditLibrary}
      onCloseCreate={closeCreateLibraryDialog}
      onCreate={createLibrary}
      onCreateDescriptionChange={setDescription}
      onCreateNameChange={setName}
      onDelete={(library) => void deleteLibrary(library)}
      onEdit={startEditLibrary}
      onEditDescriptionChange={setEditDescription}
      onEditNameChange={setEditName}
      onOpen={openLibrary}
      onOpenCreate={openCreateLibraryDialog}
      onOpenStorageCreate={openStorageDialog}
      onToggleEnabled={(library, enabled) => void toggleLibraryEnabled(library, enabled)}
      onUpdate={updateLibrary}
      onStorageConnectionChange={setStorageConnectionId}
      />
    </>
  );
}
