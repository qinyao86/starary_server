import { ArrowLeft, ImageOff, ImageUp, MoreHorizontal, Pencil, Plus, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";
import { Button } from "@/components/ui/button";
import type { ActivityItem, LibraryMember, StorageConnection, StorageRoot, TeamLibrary, TeamUser } from "../../api";
import type { TranslatorContext } from "../../types";
import { PageFrame } from "../common";
import { LibraryDialog } from "../dialogs";
import { LibraryCardList } from "./library-card-list";
import { LibraryDetailView } from "./library-detail-view";
import { LibraryIcon } from "./library-icon";

export function LibraryDetailPageView({
  activeLibrary,
  availableUsers,
  libraryActivityItems,
  libraryMembers,
  memberDialogOpen,
  memberSelections,
  assignStorageDialogOpen,
  canDeleteLibrary,
  canManageLibrary,
  token,
  iconUpdatingLibraryId,
  storageRoots,
  storageConnectionId,
  storageConnections,
  t,
  users,
  onBack,
  onDelete,
  onEdit,
  onChangeIcon,
  onClearIcon,
  onMemberDialogClose,
  onMemberDialogOpen,
  onMemberSubmit,
  onMemberSelectionChange,
  onMemberSelectionRoleChange,
  onMemberRoleUpdate,
  onRemoveMember,
  onAssignStorage,
  onAssignStorageDialogClose,
  onAssignStorageDialogOpen,
  onStorageConnectionChange,
  onOpenStorageCreate
}: TranslatorContext & {
  activeLibrary: TeamLibrary;
  availableUsers: TeamUser[];
  libraryActivityItems: ActivityItem[];
  libraryMembers: LibraryMember[];
  memberDialogOpen: boolean;
  memberSelections: Record<string, { checked: boolean; role: string }>;
  assignStorageDialogOpen: boolean;
  canDeleteLibrary: boolean;
  canManageLibrary: boolean;
  token: string | null;
  iconUpdatingLibraryId: string | null;
  storageRoots: StorageRoot[];
  storageConnectionId: string;
  storageConnections: StorageConnection[];
  users: TeamUser[];
  onBack: () => void;
  onDelete: () => void;
  onEdit: () => void;
  onChangeIcon: () => void;
  onClearIcon: () => void;
  onMemberDialogClose: () => void;
  onMemberDialogOpen: () => void;
  onMemberSubmit: (event: FormEvent) => void | Promise<void>;
  onMemberSelectionChange: (userId: string, checked: boolean) => void;
  onMemberSelectionRoleChange: (userId: string, role: string) => void;
  onMemberRoleUpdate: (member: LibraryMember, role: string) => void;
  onRemoveMember: (member: LibraryMember) => void;
  onAssignStorage: (event: FormEvent) => void | Promise<void>;
  onAssignStorageDialogClose: () => void;
  onAssignStorageDialogOpen: () => void;
  onStorageConnectionChange: (value: string) => void;
  onOpenStorageCreate: () => void;
}) {
  const [actionsOpen, setActionsOpen] = useState(false);
  const actionsRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setActionsOpen(false);
  }, [activeLibrary.id]);

  useEffect(() => {
    if (!actionsOpen) return;
    const closeOnOutsidePress = (event: PointerEvent) => {
      if (!actionsRef.current?.contains(event.target as Node)) {
        setActionsOpen(false);
      }
    };
    window.addEventListener("pointerdown", closeOnOutsidePress);
    return () => window.removeEventListener("pointerdown", closeOnOutsidePress);
  }, [actionsOpen]);

  return (
    <PageFrame
      className="library-detail-page-frame"
      title={activeLibrary.displayName}
      description=""
      titleSlot={
        <div className="library-detail-heading">
          <button
            aria-label={t("backToLibraries")}
            className="page-back-button library-detail-back-button"
            title={t("backToLibraries")}
            type="button"
            onClick={onBack}
          >
            <ArrowLeft size={16} />
          </button>
          <LibraryIcon
            busy={iconUpdatingLibraryId === activeLibrary.id}
            editable={canManageLibrary}
            library={activeLibrary}
            size="sm"
            t={t}
            token={token}
            onChange={onChangeIcon}
          />
          <h2>{activeLibrary.displayName}</h2>
        </div>
      }
      action={
        <div className="library-detail-page-actions">
          {canManageLibrary && <Button className="library-edit-button" size="sm" type="button" onClick={onEdit}>
            <Pencil size={15} />
            <span>{t("editLibrary")}</span>
          </Button>}
          {(canManageLibrary || canDeleteLibrary) && <div className="library-detail-more" ref={actionsRef}>
            <Button
              aria-expanded={actionsOpen}
              aria-haspopup="menu"
              aria-label={t("action")}
              className="library-detail-more-button"
              size="icon"
              title={t("action")}
              type="button"
              variant="ghost"
              onClick={() => setActionsOpen((open) => !open)}
            >
              <MoreHorizontal size={16} />
            </Button>
            {actionsOpen && <div className="library-detail-actions-menu" role="menu">
              {canManageLibrary && <button
                disabled={iconUpdatingLibraryId === activeLibrary.id}
                role="menuitem"
                type="button"
                onClick={() => {
                  setActionsOpen(false);
                  onChangeIcon();
                }}
              >
                <ImageUp size={15} />
                <span>{t("changeLibraryIcon")}</span>
              </button>}
              {canManageLibrary && activeLibrary.iconUrl && <button
                disabled={iconUpdatingLibraryId === activeLibrary.id}
                role="menuitem"
                type="button"
                onClick={() => {
                  setActionsOpen(false);
                  onClearIcon();
                }}
              >
                <ImageOff size={15} />
                <span>{t("clearLibraryIcon")}</span>
              </button>}
              {canDeleteLibrary && <button
                className="is-danger"
                role="menuitem"
                type="button"
                onClick={() => {
                  setActionsOpen(false);
                  onDelete();
                }}
              >
                <Trash2 size={15} />
                <span>{t("deleteLibrary")}</span>
              </button>}
            </div>}
          </div>}
        </div>
      }
    >
      <LibraryDetailView
        activeLibrary={activeLibrary}
        availableUsers={availableUsers}
        libraryActivityItems={libraryActivityItems}
        libraryMembers={libraryMembers}
        memberDialogOpen={memberDialogOpen}
        memberSelections={memberSelections}
        assignStorageDialogOpen={assignStorageDialogOpen}
        canAssignStorage={canManageLibrary && storageRoots.length === 0 && (activeLibrary.storageRootCount ?? 0) === 0}
        storageConnectionId={storageConnectionId}
        storageConnections={storageConnections}
        storageRoots={storageRoots}
        t={t}
        users={users}
        onMemberDialogClose={onMemberDialogClose}
        onMemberDialogOpen={onMemberDialogOpen}
        onMemberSubmit={onMemberSubmit}
        onMemberSelectionChange={onMemberSelectionChange}
        onMemberSelectionRoleChange={onMemberSelectionRoleChange}
        onMemberRoleUpdate={onMemberRoleUpdate}
        onRemoveMember={onRemoveMember}
        onAssignStorage={onAssignStorage}
        onAssignStorageDialogClose={onAssignStorageDialogClose}
        onAssignStorageDialogOpen={onAssignStorageDialogOpen}
        onStorageConnectionChange={onStorageConnectionChange}
        onOpenStorageCreate={onOpenStorageCreate}
      />
    </PageFrame>
  );
}

export function LibraryListPageView({
  createName,
  createAccessMode,
  editName,
  editAccessMode,
  editingLibraryId,
  libraries,
  canDeleteLibrary,
  canCreateLibrary,
  canManageLibrary,
  token,
  iconUpdatingLibraryId,
  showCreate,
  t,
  storageConnectionId,
  storageConnections,
  onCloseCreate,
  onCancelEdit,
  onCreate,
  onCreateAccessModeChange,
  onCreateNameChange,
  onDelete,
  onEditAccessModeChange,
  onEditNameChange,
  onOpenEdit,
  onChangeIcon,
  onOpen,
  onOpenCreate,
  onOpenStorageCreate,
  onToggleEnabled,
  onUpdate,
  onStorageConnectionChange
}: TranslatorContext & {
  createName: string;
  createAccessMode: "public" | "invite";
  editName: string;
  editAccessMode: "public" | "invite";
  editingLibraryId: string | null;
  libraries: TeamLibrary[];
  canDeleteLibrary: boolean;
  canCreateLibrary: boolean;
  canManageLibrary: (library: TeamLibrary) => boolean;
  token: string | null;
  iconUpdatingLibraryId: string | null;
  showCreate: boolean;
  storageConnectionId: string;
  storageConnections: StorageConnection[];
  onCloseCreate: () => void;
  onCancelEdit: () => void;
  onCreate: (event: FormEvent) => void | Promise<void>;
  onCreateAccessModeChange: (value: "public" | "invite") => void;
  onCreateNameChange: (value: string) => void;
  onDelete: (libraryId: string) => void;
  onEditAccessModeChange: (value: "public" | "invite") => void;
  onEditNameChange: (value: string) => void;
  onOpenEdit: (libraryId: string) => void;
  onChangeIcon: (libraryId: string) => void;
  onOpen: (libraryId: string) => void;
  onOpenCreate: () => void;
  onOpenStorageCreate: () => void;
  onToggleEnabled: (library: TeamLibrary, enabled: boolean) => void;
  onUpdate: (event: FormEvent) => void | Promise<void>;
  onStorageConnectionChange: (value: string) => void;
}) {
  return (
    <PageFrame
      className="library-list-page-frame"
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
          accessMode={createAccessMode}
          submitLabel={t("submit")}
          t={t}
          showStorage
          storageConnectionId={storageConnectionId}
          storageConnections={storageConnections}
          onClose={onCloseCreate}
          onAccessModeChange={onCreateAccessModeChange}
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
          accessMode={editAccessMode}
          submitLabel={t("submit")}
          t={t}
          onClose={onCancelEdit}
          onAccessModeChange={onEditAccessModeChange}
          onNameChange={onEditNameChange}
          onSubmit={onUpdate}
        />
        <LibraryCardList
          libraries={libraries}
          canDeleteLibrary={canDeleteLibrary}
          canManageLibrary={canManageLibrary}
          token={token}
          iconUpdatingLibraryId={iconUpdatingLibraryId}
          t={t}
          onDelete={onDelete}
          onOpen={onOpen}
          onEdit={onOpenEdit}
          onChangeIcon={onChangeIcon}
          onToggleEnabled={onToggleEnabled}
        />
      </div>
    </PageFrame>
  );
}
