import { useState } from "react";
import type { FormEvent } from "react";
import type { LibraryMember, TeamLibrary } from "../api";
import { api } from "../api";
import { LibraryDetailPageView, LibraryListPageView } from "../components/libraries/library-page-views";
import { useAvailableLibraryUsers } from "../hooks/use-available-library-users";
import type { PageContext } from "../types";

export function LibrariesPage({
  t, token, libraries, users, storageRoots, libraryMembers, libraryActivityItems, selectedLibraryId, setSelectedLibraryId, refreshAll, setMessage
}: PageContext) {
  const [viewLibraryId, setViewLibraryId] = useState<string | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [editingLibraryId, setEditingLibraryId] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [editName, setEditName] = useState("");
  const [editDescription, setEditDescription] = useState("");
  const [memberUserId, setMemberUserId] = useState("");
  const [memberRole, setMemberRole] = useState("viewer");
  const [memberDialogOpen, setMemberDialogOpen] = useState(false);
  const activeLibrary = libraries.find((item) => item.id === viewLibraryId) ?? null;
  const availableUsers = useAvailableLibraryUsers(users, libraryMembers, memberUserId, setMemberUserId);

  const openCreateLibraryDialog = () => {
    cancelEditLibrary();
    setName("");
    setDescription("");
    setShowCreate(true);
  };

  const closeCreateLibraryDialog = () => {
    setShowCreate(false);
    setName("");
    setDescription("");
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
  };

  const cancelEditLibrary = () => {
    setEditingLibraryId(null);
    setEditName("");
    setEditDescription("");
  };

  const openMemberDialog = () => {
    setMemberUserId(availableUsers[0]?.id ?? "");
    setMemberRole("viewer");
    setMemberDialogOpen(true);
  };

  const createLibrary = async (event: FormEvent) => {
    event.preventDefault();
    if (!name.trim()) {
      setMessage(t("formRequiredHint"));
      return;
    }
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      const library = await api.createLibrary(token, { displayName: name.trim(), description: description.trim() || undefined });
      closeCreateLibraryDialog();
      setViewLibraryId(library.id);
      setSelectedLibraryId(library.id);
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
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
        description: editDescription.trim() || undefined
      });
      cancelEditLibrary();
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
    <LibraryListPageView
      createDescription={description}
      createName={name}
      editDescription={editDescription}
      editName={editName}
      editingLibraryId={editingLibraryId}
      libraries={libraries}
      showCreate={showCreate}
      t={t}
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
      onUpdate={updateLibrary}
    />
  );
}
