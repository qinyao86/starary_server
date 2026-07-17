import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import type { LibraryMember, TeamLibrary } from "../api";
import { api } from "../api";
import { ApiError } from "../api/request";
import { DeleteLibraryDialog, LibraryDialog, StorageConnectionDialog } from "../components/dialogs";
import { LibraryDetailPageView, LibraryListPageView } from "../components/libraries/library-page-views";
import { useAvailableLibraryUsers } from "../hooks/use-available-library-users";
import type { PageContext } from "../types";
import {
  fileToLibraryIconWebpBytes,
  LibraryIconImageError,
  pickLibraryIconFile
} from "../utils/library-icons";

type PendingMemberSelection = {
  checked: boolean;
  role: string;
};

export function LibrariesPage({
  t, token, currentUser, libraries, users, storageRoots, storageConnections, libraryMembers, libraryActivityItems, selectedLibraryId, setSelectedLibraryId, refreshAll, setMessage, libraryRouteId, navigateToLibrary
}: PageContext) {
  const [showCreate, setShowCreate] = useState(false);
  const [editingLibraryId, setEditingLibraryId] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [createAccessMode, setCreateAccessMode] = useState<"public" | "invite">("invite");
  const [editAccessMode, setEditAccessMode] = useState<"public" | "invite">("invite");
  const [storageConnectionId, setStorageConnectionId] = useState("");
  const [storageDialogOpen, setStorageDialogOpen] = useState(false);
  const [assignStorageDialogOpen, setAssignStorageDialogOpen] = useState(false);
  const [storageKind, setStorageKind] = useState("server_filesystem");
  const [storageLocation, setStorageLocation] = useState("");
  const [editName, setEditName] = useState("");
  const [memberSelections, setMemberSelections] = useState<Record<string, PendingMemberSelection>>({});
  const [memberDialogOpen, setMemberDialogOpen] = useState(false);
  const [libraryPendingDeletion, setLibraryPendingDeletion] = useState<TeamLibrary | null>(null);
  const [iconUpdatingLibraryId, setIconUpdatingLibraryId] = useState<string | null>(null);
  const activeLibrary = libraries.find((item) => item.id === libraryRouteId) ?? null;
  const availableUsers = useAvailableLibraryUsers(users, libraryMembers);
  const canCreateLibrary = currentUser?.role === "owner" || currentUser?.role === "admin";
  const canManageLibrary = (library: TeamLibrary) =>
    ["owner", "admin", "library_manager"].includes(library.currentUserRole ?? currentUser?.role ?? "");

  useEffect(() => {
    if (
      !libraryRouteId ||
      selectedLibraryId === libraryRouteId ||
      !libraries.some((library) => library.id === libraryRouteId)
    ) {
      return;
    }
    setSelectedLibraryId(libraryRouteId);
  }, [libraries, libraryRouteId, selectedLibraryId, setSelectedLibraryId]);
  const storageErrorMessage = (error: unknown) => {
    if (error instanceof ApiError && error.code === "storage_location_conflict") {
      return t("storageLocationConflict");
    }
    return error instanceof Error ? error.message : String(error);
  };

  const openCreateLibraryDialog = () => {
    cancelEditLibrary();
    setName("");
    setCreateAccessMode("invite");
    setStorageConnectionId(
      storageConnections.find((connection) => connection.isDefault && connection.enabled)?.id ??
      storageConnections.find((connection) => connection.enabled)?.id ??
      ""
    );
    setShowCreate(true);
  };

  const closeCreateLibraryDialog = () => {
    setShowCreate(false);
    setName("");
    setCreateAccessMode("invite");
    setStorageConnectionId("");
  };

  const libraryIconErrorMessage = (error: unknown) => {
    if (error instanceof LibraryIconImageError) {
      return t("invalidLibraryIcon");
    }
    return error instanceof Error ? error.message : String(error);
  };

  const changeLibraryIcon = async (libraryId: string) => {
    if (!token || iconUpdatingLibraryId) return;
    const file = await pickLibraryIconFile();
    if (!file) return;

    setIconUpdatingLibraryId(libraryId);
    try {
      const imageBytes = await fileToLibraryIconWebpBytes(file);
      await api.uploadLibraryIcon(token, libraryId, imageBytes);
      await refreshAll();
      setMessage(t("libraryIconUpdated"));
    } catch (error) {
      setMessage(libraryIconErrorMessage(error));
    } finally {
      setIconUpdatingLibraryId(null);
    }
  };

  const clearLibraryIcon = async (libraryId: string) => {
    if (!token || iconUpdatingLibraryId) return;
    setIconUpdatingLibraryId(libraryId);
    try {
      await api.clearLibraryIcon(token, libraryId);
      await refreshAll();
      setMessage(t("libraryIconCleared"));
    } catch (error) {
      setMessage(libraryIconErrorMessage(error));
    } finally {
      setIconUpdatingLibraryId(null);
    }
  };

  const openLibrary = (libraryId: string) => {
    setAssignStorageDialogOpen(false);
    navigateToLibrary(libraryId);
    setSelectedLibraryId(libraryId);
  };

  const startEditLibrary = (library: TeamLibrary) => {
    setShowCreate(false);
    setEditingLibraryId(library.id);
    setEditName(library.displayName);
    setEditAccessMode(library.accessMode ?? "invite");
  };

  const startEditLibraryById = (libraryId: string) => {
    const library = libraries.find((item) => item.id === libraryId);
    if (library) startEditLibrary(library);
  };

  const startDeleteLibraryById = (libraryId: string) => {
    const library = libraries.find((item) => item.id === libraryId);
    if (library) setLibraryPendingDeletion(library);
  };

  const cancelEditLibrary = () => {
    setEditingLibraryId(null);
    setEditName("");
    setEditAccessMode("invite");
  };

  const openStorageDialog = () => {
    setStorageKind("server_filesystem");
    setStorageLocation("");
    setStorageDialogOpen(true);
  };

  const openAssignStorageDialog = () => {
    setStorageConnectionId(
      storageConnections.find((connection) => connection.isDefault && connection.enabled)?.id ??
      storageConnections.find((connection) => connection.enabled)?.id ??
      ""
    );
    setAssignStorageDialogOpen(true);
  };

  const closeAssignStorageDialog = () => {
    setAssignStorageDialogOpen(false);
    setStorageConnectionId("");
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
    setMemberSelections(Object.fromEntries(
      availableUsers.map((user) => [user.id, { checked: false, role: "editor" }])
    ));
    setMemberDialogOpen(true);
  };

  const closeMemberDialog = () => {
    setMemberDialogOpen(false);
    setMemberSelections({});
  };

  const togglePendingMember = (userId: string, checked: boolean) => {
    setMemberSelections((previous) => ({
      ...previous,
      [userId]: {
        checked,
        role: previous[userId]?.role ?? "editor"
      }
    }));
  };

  const updatePendingMemberRole = (userId: string, role: string) => {
    setMemberSelections((previous) => ({
      ...previous,
      [userId]: {
        checked: previous[userId]?.checked ?? false,
        role
      }
    }));
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
        accessMode: createAccessMode,
        storageBinding: { connectionId: storageConnectionId }
      });
      closeCreateLibraryDialog();
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(storageErrorMessage(error));
    }
  };

  const updateLibrary = async (event: FormEvent) => {
    event.preventDefault();
    if (!editingLibraryId || !editName.trim()) {
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
        accessMode: editAccessMode
      });
      cancelEditLibrary();
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(storageErrorMessage(error));
    }
  };

  const assignLibraryStorage = async (event: FormEvent) => {
    event.preventDefault();
    if (!activeLibrary || !storageConnectionId) {
      setMessage(t("formRequiredHint"));
      return;
    }
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.assignLibraryStorage(token, activeLibrary.id, { connectionId: storageConnectionId });
      closeAssignStorageDialog();
      setMessage(t("storageAssigned"));
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

  const deleteLibrary = async (deleteFiles: boolean) => {
    const library = libraryPendingDeletion;
    if (!library) return;
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.deleteLibrary(token, library.id, { deleteFiles });
      if (libraryRouteId === library.id) navigateToLibrary(null, { replace: true });
      if (selectedLibraryId === library.id) setSelectedLibraryId("");
      if (editingLibraryId === library.id) cancelEditLibrary();
      setLibraryPendingDeletion(null);
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const updateMemberRole = async (member: LibraryMember, role: string) => {
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.upsertLibraryMember(token, member.libraryId, member.userId, { role });
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const submitMember = async (event: FormEvent) => {
    event.preventDefault();
    const selectedUsers = availableUsers.filter((user) => memberSelections[user.id]?.checked);
    if (!selectedLibraryId || selectedUsers.length === 0) {
      setMessage(t("formRequiredHint"));
      return;
    }
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await Promise.all(selectedUsers.map((user) => (
        api.upsertLibraryMember(token, selectedLibraryId, user.id, {
          role: memberSelections[user.id]?.role ?? "editor"
        })
      )));
      closeMemberDialog();
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
      <>
        <DeleteLibraryDialog
          library={libraryPendingDeletion}
          open={Boolean(libraryPendingDeletion)}
          t={t}
          onClose={() => setLibraryPendingDeletion(null)}
          onConfirm={deleteLibrary}
        />
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
        <LibraryDialog
          open={Boolean(editingLibraryId) && !storageDialogOpen}
          title={t("updateLibrary")}
          hint={t("editLibraryDialogHint")}
          name={editName}
          accessMode={editAccessMode}
          submitLabel={t("submit")}
          t={t}
          onClose={cancelEditLibrary}
          onNameChange={setEditName}
          onAccessModeChange={setEditAccessMode}
          onSubmit={updateLibrary}
        />
        <LibraryDetailPageView
          activeLibrary={activeLibrary}
          availableUsers={availableUsers}
          libraryActivityItems={libraryActivityItems}
          libraryMembers={libraryMembers}
          memberDialogOpen={memberDialogOpen}
          memberSelections={memberSelections}
          canDeleteLibrary={canCreateLibrary}
          canManageLibrary={canManageLibrary(activeLibrary)}
          token={token}
          iconUpdatingLibraryId={iconUpdatingLibraryId}
          assignStorageDialogOpen={assignStorageDialogOpen && !storageDialogOpen}
          storageConnectionId={storageConnectionId}
          storageConnections={storageConnections}
          storageRoots={storageRoots}
          t={t}
          users={users}
          onBack={() => {
            closeAssignStorageDialog();
            navigateToLibrary(null);
          }}
          onDelete={() => setLibraryPendingDeletion(activeLibrary)}
          onEdit={() => startEditLibrary(activeLibrary)}
          onChangeIcon={() => void changeLibraryIcon(activeLibrary.id)}
          onClearIcon={() => void clearLibraryIcon(activeLibrary.id)}
          onMemberDialogClose={closeMemberDialog}
          onMemberDialogOpen={openMemberDialog}
          onMemberSubmit={submitMember}
          onMemberSelectionChange={togglePendingMember}
          onMemberSelectionRoleChange={updatePendingMemberRole}
          onMemberRoleUpdate={(member, role) => void updateMemberRole(member, role)}
          onRemoveMember={(member) => void removeMember(member)}
          onAssignStorage={assignLibraryStorage}
          onAssignStorageDialogClose={closeAssignStorageDialog}
          onAssignStorageDialogOpen={openAssignStorageDialog}
          onStorageConnectionChange={setStorageConnectionId}
          onOpenStorageCreate={openStorageDialog}
        />
      </>
    );
  }

  return (
    <>
      <DeleteLibraryDialog
        library={libraryPendingDeletion}
        open={Boolean(libraryPendingDeletion)}
        t={t}
        onClose={() => setLibraryPendingDeletion(null)}
        onConfirm={deleteLibrary}
      />
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
      createName={name}
      createAccessMode={createAccessMode}
      editName={editName}
      editAccessMode={editAccessMode}
      editingLibraryId={storageDialogOpen ? null : editingLibraryId}
      libraries={libraries}
      canDeleteLibrary={canCreateLibrary}
      canCreateLibrary={canCreateLibrary}
      canManageLibrary={canManageLibrary}
      token={token}
      iconUpdatingLibraryId={iconUpdatingLibraryId}
      showCreate={showCreate && !storageDialogOpen}
      t={t}
      storageConnectionId={storageConnectionId}
      storageConnections={storageConnections}
      onCancelEdit={cancelEditLibrary}
      onCloseCreate={closeCreateLibraryDialog}
      onCreate={createLibrary}
      onCreateNameChange={setName}
      onCreateAccessModeChange={setCreateAccessMode}
      onDelete={startDeleteLibraryById}
      onEditNameChange={setEditName}
      onEditAccessModeChange={setEditAccessMode}
      onOpenEdit={startEditLibraryById}
      onChangeIcon={(libraryId) => void changeLibraryIcon(libraryId)}
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
