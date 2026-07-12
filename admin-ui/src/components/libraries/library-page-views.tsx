import { ArrowLeft, Pencil, Plus, Trash2 } from "lucide-react";
import type { FormEvent } from "react";
import { Button } from "@/components/ui/button";
import type { ActivityItem, LibraryMember, StorageConnection, StorageRoot, TeamLibrary, TeamUser } from "../../api";
import type { TranslatorContext } from "../../types";
import { PageFrame } from "../common";
import { LibraryDialog } from "../dialogs";
import { LibraryCardList } from "./library-card-list";
import { LibraryDetailView } from "./library-detail-view";

export function LibraryDetailPageView({
  activeLibrary,
  availableUsers,
  libraryActivityItems,
  libraryMembers,
  memberDialogOpen,
  memberRole,
  memberUserId,
  canDeleteLibrary,
  canManageLibrary,
  storageRoots,
  t,
  onBack,
  onDelete,
  onEdit,
  onMemberDialogClose,
  onMemberDialogOpen,
  onMemberRoleChange,
  onMemberSubmit,
  onMemberUserChange,
  onMemberRoleUpdate,
  onRemoveMember
}: TranslatorContext & {
  activeLibrary: TeamLibrary;
  availableUsers: TeamUser[];
  libraryActivityItems: ActivityItem[];
  libraryMembers: LibraryMember[];
  memberDialogOpen: boolean;
  memberRole: string;
  memberUserId: string;
  canDeleteLibrary: boolean;
  canManageLibrary: boolean;
  storageRoots: StorageRoot[];
  onBack: () => void;
  onDelete: () => void;
  onEdit: () => void;
  onMemberDialogClose: () => void;
  onMemberDialogOpen: () => void;
  onMemberRoleChange: (value: string) => void;
  onMemberSubmit: (event: FormEvent) => void | Promise<void>;
  onMemberUserChange: (value: string) => void;
  onMemberRoleUpdate: (member: LibraryMember, role: string) => void;
  onRemoveMember: (member: LibraryMember) => void;
}) {
  return (
    <PageFrame
      title={t("libraries")}
      description={t("libraryPageHint")}
      titleSlot={
        <button className="page-back-button" type="button" onClick={onBack}>
          <ArrowLeft size={16} />
          <span>{t("backToLibraries")}</span>
        </button>
      }
      action={
        <div className="library-detail-page-actions">
          {canManageLibrary && <Button size="sm" type="button" variant="outline" onClick={onEdit}>
            <Pencil size={15} />
            <span>{t("editLibrary")}</span>
          </Button>}
          {canDeleteLibrary && <Button size="sm" type="button" variant="destructive" onClick={onDelete}>
            <Trash2 size={15} />
            <span>{t("deleteLibrary")}</span>
          </Button>}
        </div>
      }
    >
      <LibraryDetailView
        activeLibrary={activeLibrary}
        availableUsers={availableUsers}
        libraryActivityItems={libraryActivityItems}
        libraryMembers={libraryMembers}
        memberDialogOpen={memberDialogOpen}
        memberRole={memberRole}
        memberUserId={memberUserId}
        storageRoots={storageRoots}
        t={t}
        onMemberDialogClose={onMemberDialogClose}
        onMemberDialogOpen={onMemberDialogOpen}
        onMemberRoleChange={onMemberRoleChange}
        onMemberSubmit={onMemberSubmit}
        onMemberUserChange={onMemberUserChange}
        onMemberRoleUpdate={onMemberRoleUpdate}
        onRemoveMember={onRemoveMember}
      />
    </PageFrame>
  );
}

export function LibraryListPageView({
  createName,
  editName,
  editingLibraryId,
  editingStorageLocked,
  libraries,
  canCreateLibrary,
  canManageLibrary,
  showCreate,
  t,
  storageConnectionId,
  storageConnections,
  onCloseCreate,
  onCancelEdit,
  onCreate,
  onCreateNameChange,
  onEditNameChange,
  onOpen,
  onOpenCreate,
  onOpenStorageCreate,
  onToggleEnabled,
  onUpdate,
  onStorageConnectionChange
}: TranslatorContext & {
  createName: string;
  editName: string;
  editingLibraryId: string | null;
  editingStorageLocked: boolean;
  libraries: TeamLibrary[];
  canCreateLibrary: boolean;
  canManageLibrary: (library: TeamLibrary) => boolean;
  showCreate: boolean;
  storageConnectionId: string;
  storageConnections: StorageConnection[];
  onCloseCreate: () => void;
  onCancelEdit: () => void;
  onCreate: (event: FormEvent) => void | Promise<void>;
  onCreateNameChange: (value: string) => void;
  onEditNameChange: (value: string) => void;
  onOpen: (libraryId: string) => void;
  onOpenCreate: () => void;
  onOpenStorageCreate: () => void;
  onToggleEnabled: (library: TeamLibrary, enabled: boolean) => void;
  onUpdate: (event: FormEvent) => void | Promise<void>;
  onStorageConnectionChange: (value: string) => void;
}) {
  return (
    <PageFrame
      title={t("libraries")}
      description={t("libraryPageHint")}
      action={canCreateLibrary ? (
        <Button type="button" onClick={onOpenCreate}>
          <Plus size={16} />
          <span>{t("createLibrary")}</span>
        </Button>
      ) : undefined}
    >
      <div className="library-page">
        <LibraryDialog
          open={showCreate}
          title={t("createLibrary")}
          hint={t("createLibraryDialogHint")}
          name={createName}
          submitLabel={t("submit")}
          t={t}
          showStorage
          storageConnectionId={storageConnectionId}
          storageConnections={storageConnections}
          onClose={onCloseCreate}
          onNameChange={onCreateNameChange}
          onSubmit={onCreate}
          onStorageConnectionChange={onStorageConnectionChange}
          onCreateStorage={onOpenStorageCreate}
        />
        <LibraryDialog
          open={Boolean(editingLibraryId)}
          title={t("updateLibrary")}
          hint={t("editLibraryDialogHint")}
          name={editName}
          submitLabel={t("submit")}
          t={t}
          showStorage
          storageLocked={editingStorageLocked}
          storageConnectionId={storageConnectionId}
          storageConnections={storageConnections}
          onClose={onCancelEdit}
          onNameChange={onEditNameChange}
          onStorageConnectionChange={onStorageConnectionChange}
          onCreateStorage={onOpenStorageCreate}
          onSubmit={onUpdate}
        />
        <LibraryCardList
          libraries={libraries}
          canManageLibrary={canManageLibrary}
          t={t}
          onOpen={onOpen}
          onToggleEnabled={onToggleEnabled}
        />
      </div>
    </PageFrame>
  );
}
