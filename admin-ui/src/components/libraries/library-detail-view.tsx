import { ListChecks, Network, Trash2, UserPlus, Users } from "lucide-react";
import type { FormEvent } from "react";
import { Badge as UiBadge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { ActivityItem, LibraryMember, StorageRoot, TeamLibrary, TeamUser } from "../../api";
import type { TranslatorContext } from "../../types";
import { formatBytes, formatCount, isLibraryManagerRole, roleLabel } from "../../utils/format";
import { ActivityList, Badge, DataTable, LibraryDetailStat, Panel } from "../common";
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
          <CardDescription>{activeLibrary.description || t("noDescription")}</CardDescription>
        </CardHeader>
        <CardContent className="library-detail-stats p-0">
          <LibraryDetailStat label={t("totalSize")} value={formatBytes(activeLibrary.totalSizeBytes)} />
          <LibraryDetailStat label={t("members")} value={formatCount(visibleMemberCount)} />
          <LibraryDetailStat label={t("assets")} value={formatCount(activeLibrary.assetCount)} />
          <LibraryDetailStat label={t("tags")} value={formatCount(activeLibrary.tagCount)} />
          <LibraryDetailStat label={t("creator")} value={activeLibrary.creatorName || "-"} />
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
            onRemoveMember={onRemoveMember}
          />
        </Panel>

        <Panel title={t("libraryStorage")} icon={Network} className="span-6" action={<Badge>{storageRoots.length ? t("realData") : t("empty")}</Badge>}>
          <DataTable
            emptyLabel={t("noSharedRoots")}
            columns={[t("name"), t("provider"), t("status")]}
            rows={storageRoots.map((root) => [root.name, root.kind, root.enabled ? t("enabled") : t("disabled")])}
          />
        </Panel>
        <Panel title={t("libraryActivity")} icon={ListChecks} className="span-6" action={<Badge>{libraryActivityItems.length ? t("realData") : t("empty")}</Badge>}>
          <ActivityList t={t} activityItems={libraryActivityItems} compact />
        </Panel>
      </div>
    </div>
  );
}

function LibraryMemberList({
  activeLibrary,
  libraryManagerCount,
  libraryMembers,
  t,
  onRemoveMember
}: TranslatorContext & {
  activeLibrary: TeamLibrary;
  libraryManagerCount: number;
  libraryMembers: LibraryMember[];
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
        return (
          <div className="member-row" key={member.userId}>
            <div>
              <strong>{member.displayName}</strong>
              <span>{member.email}</span>
            </div>
            <UiBadge className="member-role-badge" variant="secondary">{roleLabel(t, member.role)}</UiBadge>
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
