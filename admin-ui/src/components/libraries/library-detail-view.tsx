import { ChevronDown, ListChecks, Network, Trash2, UserPlus, Users } from "lucide-react";
import type { FormEvent } from "react";
import { Badge as UiBadge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { ActivityItem, LibraryMember, StorageRoot, TeamLibrary, TeamUser } from "../../api";
import type { TranslatorContext } from "../../types";
import { formatBytes, formatCount, formatMemberNames, isLibraryManagerRole, roleLabel, storageKindLabel } from "../../utils/format";
import { ActivityList, Badge, LibraryDetailStat, Panel } from "../common";
import { MemberDialog } from "../dialogs";

export function LibraryDetailView({
  activeLibrary,
  availableUsers,
  libraryActivityItems,
  libraryMembers,
  memberDialogOpen,
  memberRole,
  memberUserId,
  storageRoots,
  t,
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
  storageRoots: StorageRoot[];
  onMemberDialogClose: () => void;
  onMemberDialogOpen: () => void;
  onMemberRoleChange: (value: string) => void;
  onMemberSubmit: (event: FormEvent) => void | Promise<void>;
  onMemberUserChange: (value: string) => void;
  onMemberRoleUpdate: (member: LibraryMember, role: string) => void;
  onRemoveMember: (member: LibraryMember) => void;
}) {
  const libraryManagerCount = libraryMembers.filter((member) => isLibraryManagerRole(member.role)).length;
  const visibleMemberCount = Math.max(libraryMembers.length, activeLibrary.memberNames?.length ?? 0);

  return (
    <div className="library-page">
      <MemberDialog
        open={memberDialogOpen}
        t={t}
        users={availableUsers}
        memberUserId={memberUserId}
        memberRole={memberRole}
        onClose={onMemberDialogClose}
        onRoleChange={onMemberRoleChange}
        onSubmit={onMemberSubmit}
        onUserChange={onMemberUserChange}
      />

      <Card className="library-detail-hero">
        <CardHeader className="p-0">
          <CardDescription>{t("libraryDetails")}</CardDescription>
          <CardTitle className="text-[22px]">{activeLibrary.displayName}</CardTitle>
        </CardHeader>
        <CardContent className="library-detail-stats p-0">
          <LibraryDetailStat label={t("totalSize")} value={formatBytes(activeLibrary.totalSizeBytes)} />
          <LibraryDetailStat label={t("members")} value={formatCount(visibleMemberCount)} />
          <LibraryDetailStat label={t("assets")} value={formatCount(activeLibrary.assetCount)} />
          <LibraryDetailStat label={t("tags")} value={formatCount(activeLibrary.tagCount)} />
          <LibraryDetailStat label={t("libraryManager")} value={formatMemberNames(activeLibrary.libraryManagerNames, "-")} />
          <LibraryDetailStat label={t("role")} value={roleLabel(t, activeLibrary.currentUserRole ?? "owner")} />
        </CardContent>
      </Card>

      <div className="page-grid">
        <Panel
          title={t("libraryMembers")}
          icon={Users}
          className="span-12"
          action={
            <div className="panel-actions">
              <UiBadge className="control-badge" variant="secondary">{libraryMembers.length ? t("realData") : t("empty")}</UiBadge>
              <Button className="panel-action-button" size="sm" type="button" onClick={onMemberDialogOpen} disabled={availableUsers.length === 0}>
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

        <Panel title={t("storage")} icon={Network} className="span-6" action={<Badge>{storageRoots.length ? t("realData") : t("empty")}</Badge>}>
          <LibraryStorageList storageRoots={storageRoots} t={t} />
        </Panel>
        <Panel title={t("libraryActivity")} icon={ListChecks} className="span-6" action={<Badge>{libraryActivityItems.length ? t("realData") : t("empty")}</Badge>}>
          <ActivityList t={t} activityItems={libraryActivityItems} compact />
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
              className="member-action-button"
              size="sm"
              type="button"
              variant="outline"
              onClick={() => onRemoveMember(member)}
              disabled={removingWouldOrphanLibrary}
            >
              <Trash2 size={15} />
              <span>{t("remove")}</span>
            </Button>
          </div>
        );
      })}
    </div>
  );
}
