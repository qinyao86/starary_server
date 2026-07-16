import { ChevronDown, Database, HardDrive, History, Package2, Trash2, UserPlus, Users } from "lucide-react";
import type { FormEvent } from "react";
import { Badge as UiBadge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { ActivityItem, LibraryMember, StorageConnection, StorageRoot, TeamLibrary, TeamUser } from "../../api";
import type { TranslatorContext } from "../../types";
import { formatBytes, formatCount, isLibraryManagerRole, roleLabel, storageKindLabel } from "../../utils/format";
import { ActivityList, LibraryDetailStat, Panel, UserAvatar } from "../common";
import { LibraryDialog, MemberDialog } from "../dialogs";

export function LibraryDetailView({
  activeLibrary,
  availableUsers,
  libraryActivityItems,
  libraryMembers,
  memberDialogOpen,
  memberSelections,
  assignStorageDialogOpen,
  canAssignStorage,
  storageConnectionId,
  storageConnections,
  storageRoots,
  t,
  users,
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
  canAssignStorage: boolean;
  storageConnectionId: string;
  storageConnections: StorageConnection[];
  storageRoots: StorageRoot[];
  users: TeamUser[];
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
  const libraryManagerCount = libraryMembers.filter((member) => isLibraryManagerRole(member.role)).length;
  const visibleMemberCount = Math.max(libraryMembers.length, activeLibrary.memberNames?.length ?? 0);
  const resolveAvatarKey = (item: ActivityItem) => {
    const user = users.find((candidate) =>
      candidate.id === item.actorUserId ||
      candidate.email.toLowerCase() === item.actorEmail?.toLowerCase()
    );
    return user?.avatarKey ?? item.actorAvatarKey ?? null;
  };

  return (
    <div className="library-page">
      <MemberDialog
        open={memberDialogOpen}
        t={t}
        users={availableUsers}
        memberSelections={memberSelections}
        onClose={onMemberDialogClose}
        onSubmit={onMemberSubmit}
        onSelectionChange={onMemberSelectionChange}
        onRoleChange={onMemberSelectionRoleChange}
      />

      <LibraryDialog
        hint={t("assignLibraryStorageHint")}
        name=""
        open={canAssignStorage && assignStorageDialogOpen}
        showName={false}
        showStorage
        storageConnectionId={storageConnectionId}
        storageConnections={storageConnections}
        submitLabel={t("assignStorage")}
        t={t}
        title={t("assignStorage")}
        onClose={onAssignStorageDialogClose}
        onCreateStorage={onOpenStorageCreate}
        onNameChange={() => undefined}
        onStorageConnectionChange={onStorageConnectionChange}
        onSubmit={onAssignStorage}
      />

      <div className="library-detail-layout">
        <div className="library-detail-main-column">
          <section className="library-detail-overview" aria-label={t("libraryOverview")}>
            <div className="library-detail-stats">
              <LibraryDetailStat icon={Package2} label={t("assets")} value={formatCount(activeLibrary.assetCount)} />
              <LibraryDetailStat icon={Database} label={t("totalSize")} value={formatBytes(activeLibrary.totalSizeBytes)} />
              <LibraryDetailStat icon={Users} label={t("members")} value={formatCount(visibleMemberCount)} />
            </div>
          </section>

          <Panel
            title={t("storage")}
            icon={HardDrive}
            action={canAssignStorage ? (
              <Button className="panel-action-button" size="sm" type="button" variant="secondary" onClick={onAssignStorageDialogOpen}>
                <HardDrive size={15} />
                <span>{t("configureStorage")}</span>
              </Button>
            ) : undefined}
          >
            <LibraryStorageList storageRoots={storageRoots} t={t} />
          </Panel>

          <Panel
            title={t("members")}
            icon={Users}
            action={
              <div className="panel-actions">
                <UiBadge className="control-badge" title={t("members")} variant="secondary">{formatCount(visibleMemberCount)}</UiBadge>
                <Button className="panel-action-button" size="sm" type="button" variant="secondary" onClick={onMemberDialogOpen} disabled={availableUsers.length === 0}>
                  <UserPlus size={15} />
                  <span>{t("addMember")}</span>
                </Button>
              </div>
            }
          >
            <LibraryMemberList
              activeLibrary={activeLibrary}
              libraryManagerCount={libraryManagerCount}
              libraryMembers={libraryMembers}
              t={t}
              onMemberRoleUpdate={onMemberRoleUpdate}
              onRemoveMember={onRemoveMember}
            />
          </Panel>
        </div>

        <Panel title={t("recentLibraryActivity")} icon={History} className="library-detail-activity-panel">
          <ActivityList t={t} activityItems={libraryActivityItems} compact resolveAvatarKey={resolveAvatarKey} />
        </Panel>
      </div>
    </div>
  );
}

function preferredStoragePath(root: StorageRoot) {
  const platform = `${navigator.platform} ${navigator.userAgent}`.toLowerCase();
  if (platform.includes("mac")) return root.macosSmbUrl || root.canonicalUri || root.windowsUncPath || "-";
  if (platform.includes("win")) return root.windowsUncPath || root.canonicalUri || root.macosSmbUrl || "-";
  return root.canonicalUri || root.macosSmbUrl || root.windowsUncPath || "-";
}

function LibraryStorageList({ storageRoots, t }: { storageRoots: StorageRoot[]; t: TranslatorContext["t"] }) {
  if (storageRoots.length === 0) {
    return <div className="placeholder-box">{t("noSharedRoots")}</div>;
  }

  return (
    <div className="library-detail-storage-table">
      <table className="data-table">
        <thead>
          <tr>
            <th>{t("storageLocation")}</th>
            <th>{t("kind")}</th>
            <th>{t("status")}</th>
          </tr>
        </thead>
        <tbody>
          {storageRoots.map((root) => {
            const path = preferredStoragePath(root);
            return (
              <tr key={root.id}>
                <td><strong className="library-detail-storage-path" title={path}>{path}</strong></td>
                <td>{storageKindLabel(t, root.kind)}</td>
                <td>{root.enabled ? t("enabled") : t("disabled")}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

function LibraryMemberList({
  activeLibrary,
  libraryManagerCount,
  libraryMembers,
  t,
  onMemberRoleUpdate,
  onRemoveMember
}: TranslatorContext & {
  activeLibrary: TeamLibrary;
  libraryManagerCount: number;
  libraryMembers: LibraryMember[];
  onMemberRoleUpdate: (member: LibraryMember, role: string) => void;
  onRemoveMember: (member: LibraryMember) => void;
}) {
  if (libraryMembers.length === 0 && activeLibrary.memberNames?.length) {
    return (
      <div className="member-list">
        {activeLibrary.memberNames.map((name) => (
          <div className="member-row is-readonly" key={name}>
            <UserAvatar label={name} size="lg" />
            <div>
              <strong>{name}</strong>
              <span>{t("readOnly")}</span>
            </div>
          </div>
        ))}
      </div>
    );
  }

  if (libraryMembers.length === 0) {
    return <div className="placeholder-box">{t("noMembers")}</div>;
  }

  return (
    <div className="member-list">
      {libraryMembers.map((member) => {
        const removingWouldOrphanLibrary = isLibraryManagerRole(member.role) && libraryManagerCount <= 1;
        const roleLocked = removingWouldOrphanLibrary || member.role === "owner" || member.role === "admin";
        return (
          <div className="member-row" key={member.userId}>
            <UserAvatar avatarKey={member.avatarKey} label={member.displayName} size="lg" />
            <div>
              <strong>{member.displayName}</strong>
              <span>{member.email}</span>
            </div>
            <label className="member-role-control">
              <select
                aria-label={t("role")}
                disabled={roleLocked}
                value={member.role}
                onChange={(event) => onMemberRoleUpdate(member, event.target.value)}
              >
                {(member.role === "owner" || member.role === "admin") && <option value={member.role}>{roleLabel(t, member.role)}</option>}
                <option value="library_manager">{t("manager")}</option>
                <option value="editor">{t("editor")}</option>
                <option value="viewer">{t("viewer")}</option>
              </select>
              <ChevronDown aria-hidden="true" size={14} />
            </label>
            <Button
              aria-label={t("remove")}
              className="member-action-button icon-only"
              size="icon"
              title={t("remove")}
              type="button"
              variant="ghost"
              onClick={() => onRemoveMember(member)}
              disabled={removingWouldOrphanLibrary}
            >
              <Trash2 size={15} />
            </Button>
          </div>
        );
      })}
    </div>
  );
}
