import { ArrowLeft, Plus } from "lucide-react";
import type { FormEvent } from "react";
import { Button } from "@/components/ui/button";
import type { ActivityItem, LibraryMember, StorageRoot, TeamLibrary, TeamUser } from "../../api";
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
  storageRoots,
  t,
  onBack,
  onMemberDialogClose,
  onMemberDialogOpen,
  onMemberRoleChange,
  onMemberSubmit,
  onMemberUserChange,
  onRemoveMember
}: TranslatorContext & {
  activeLibrary: TeamLibrary;
  availableUsers: TeamUser[];
  libraryActivityItems: ActivityItem[];
  libraryMembers: LibraryMember[];
  memberDialogOpen: boolean;
  memberRole: string;
  memberUserId: string;
  storageRoots: StorageRoot[];
  onBack: () => void;
  onMemberDialogClose: () => void;
  onMemberDialogOpen: () => void;
  onMemberRoleChange: (value: string) => void;
  onMemberSubmit: (event: FormEvent) => void | Promise<void>;
  onMemberUserChange: (value: string) => void;
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
        onRemoveMember={onRemoveMember}
      />
    </PageFrame>
  );
}

export function LibraryListPageView({
  createDescription,
  createName,
  editDescription,
  editName,
  editingLibraryId,
  libraries,
  showCreate,
  t,
  workspaceCanonicalUri,
  workspaceKind,
  onCloseCreate,
  onCancelEdit,
  onCreate,
  onCreateDescriptionChange,
  onCreateNameChange,
  onDelete,
  onEdit,
  onEditDescriptionChange,
  onEditNameChange,
  onOpen,
  onOpenCreate,
  onToggleEnabled,
  onUpdate,
  onWorkspaceCanonicalUriChange,
  onWorkspaceKindChange
}: TranslatorContext & {
  createDescription: string;
  createName: string;
  editDescription: string;
  editName: string;
  editingLibraryId: string | null;
  libraries: TeamLibrary[];
  showCreate: boolean;
  workspaceCanonicalUri: string;
  workspaceKind: string;
  onCloseCreate: () => void;
  onCancelEdit: () => void;
  onCreate: (event: FormEvent) => void | Promise<void>;
  onCreateDescriptionChange: (value: string) => void;
  onCreateNameChange: (value: string) => void;
  onDelete: (library: TeamLibrary) => void;
  onEdit: (library: TeamLibrary) => void;
  onEditDescriptionChange: (value: string) => void;
  onEditNameChange: (value: string) => void;
  onOpen: (libraryId: string) => void;
  onOpenCreate: () => void;
  onToggleEnabled: (library: TeamLibrary, enabled: boolean) => void;
  onUpdate: (event: FormEvent) => void | Promise<void>;
  onWorkspaceCanonicalUriChange: (value: string) => void;
  onWorkspaceKindChange: (value: string) => void;
}) {
  return (
    <PageFrame
      title={t("libraries")}
      description={t("libraryPageHint")}
      action={
        <Button type="button" onClick={onOpenCreate}>
          <Plus size={16} />
          <span>{t("createLibrary")}</span>
        </Button>
      }
    >
      <div className="library-page">
        <LibraryDialog
          open={showCreate}
          title={t("createLibrary")}
          hint={t("libraryPageHint")}
          name={createName}
          description={createDescription}
          submitLabel={t("submit")}
          t={t}
          showWorkspaceSection
          workspaceCanonicalUri={workspaceCanonicalUri}
          workspaceKind={workspaceKind}
          onClose={onCloseCreate}
          onDescriptionChange={onCreateDescriptionChange}
          onNameChange={onCreateNameChange}
          onSubmit={onCreate}
          onWorkspaceCanonicalUriChange={onWorkspaceCanonicalUriChange}
          onWorkspaceKindChange={onWorkspaceKindChange}
        />
        <LibraryDialog
          open={Boolean(editingLibraryId)}
          title={t("updateLibrary")}
          hint={t("libraryPageHint")}
          name={editName}
          description={editDescription}
          submitLabel={t("submit")}
          t={t}
          onClose={onCancelEdit}
          onDescriptionChange={onEditDescriptionChange}
          onNameChange={onEditNameChange}
          onSubmit={onUpdate}
        />
        <LibraryCardList libraries={libraries} t={t} onDelete={onDelete} onEdit={onEdit} onOpen={onOpen} onToggleEnabled={onToggleEnabled} />
      </div>
    </PageFrame>
  );
}
