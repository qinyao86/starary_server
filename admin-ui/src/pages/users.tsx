import { useMemo, useState } from "react";
import { Pencil, Plus, Power, Search, Trash2, UserPlus } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { SystemAvatar, TeamUser } from "../api";
import { api } from "../api";
import type { PageContext } from "../types";
import { PageFrame, StatusDot, UserAvatar } from "../components/common";
import { AvatarDialog, DeleteUserDialog, UserDialog, UserLibraryAccessDialog } from "../components/dialogs";
import { defaultNewUserPassword } from "../constants";
import { defaultSystemAvatars } from "../utils/avatars";
import { canManageServerRole, formatDateTime, isUserOnline, roleLabel } from "../utils/format";

function emailPrefix(value: string) {
  return value.trim().split("@", 1)[0] ?? "";
}

export function UsersPage({ t, token, users, currentUser, libraries, refreshAll, setMessage }: PageContext) {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingUser, setEditingUser] = useState<TeamUser | null>(null);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [role, setRole] = useState("viewer");
  const [isActive, setIsActive] = useState(true);
  const [libraryRoles, setLibraryRoles] = useState<Record<string, string>>({});
  const [libraryAccessManageOpen, setLibraryAccessManageOpen] = useState(false);
  const [libraryAccessTarget, setLibraryAccessTarget] = useState<TeamUser | null>(null);
  const [query, setQuery] = useState("");
  const [avatarTarget, setAvatarTarget] = useState<TeamUser | null>(null);
  const [avatars, setAvatars] = useState<SystemAvatar[]>(defaultSystemAvatars);
  const [avatarBusy, setAvatarBusy] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<TeamUser | null>(null);
  const [deleteBusy, setDeleteBusy] = useState(false);

  const canManageUsers = canManageServerRole(currentUser?.role ?? "");
  const canManageAccountIdentity = currentUser?.role === "owner";
  const onlineUserCount = useMemo(() => users.filter((item) => isUserOnline(item)).length, [users]);
  const filteredUsers = useMemo(() => {
    const value = query.trim().toLocaleLowerCase();
    if (!value) return users;
    return users.filter((item) =>
      [
        item.displayName,
        item.email,
        roleLabel(t, item.globalRole),
        item.isActive ? t("enabled") : t("disabled"),
        isUserOnline(item) ? t("online") : t("offline"),
        item.lastSeenLibraryName ?? "",
        ...(item.libraryMemberships ?? []).flatMap((membership) => [membership.libraryName, roleLabel(t, membership.role)])
      ]
        .join(" ")
        .toLocaleLowerCase()
        .includes(value)
    );
  }, [query, t, users]);

  const openCreateDialog = () => {
    setEditingUser(null);
    setEmail("");
    setPassword("");
    setDisplayName("");
    setRole("viewer");
    setIsActive(true);
    setLibraryRoles({});
    setDialogOpen(true);
  };

  const openEditDialog = (user: TeamUser) => {
    setEditingUser(user);
    setEmail(user.email);
    setPassword("");
    setDisplayName(user.displayName);
    setRole(user.globalRole === "owner" || user.globalRole === "admin" ? user.globalRole : "viewer");
    setIsActive(user.isActive);
    setLibraryRoles(Object.fromEntries((user.libraryMemberships ?? []).map((membership) => [membership.libraryId, membership.role])));
    setDialogOpen(true);
  };

  const userLibraryRoles = (user: TeamUser) =>
    Object.fromEntries((user.libraryMemberships ?? []).map((membership) => [membership.libraryId, membership.role]));

  const libraryMembershipsFromRoles = (roles: Record<string, string>) =>
    libraries.flatMap((library) => {
      const membershipRole = roles[library.id] ?? "";
      return membershipRole ? [{ libraryId: library.id, role: membershipRole }] : [];
    });

  const openLibraryAccessDialog = (user: TeamUser) => {
    setLibraryAccessTarget(user);
    setLibraryRoles(userLibraryRoles(user));
    setLibraryAccessManageOpen(true);
  };

  const closeLibraryAccessDialog = () => {
    setLibraryAccessManageOpen(false);
    setLibraryAccessTarget(null);
  };

  const closeDialog = () => {
    setDialogOpen(false);
    setLibraryAccessManageOpen(false);
    setLibraryAccessTarget(null);
  };

  const saveLibraryAccess = async (roles: Record<string, string>) => {
    if (!libraryAccessTarget) {
      setLibraryRoles(roles);
      return true;
    }
    if (!token) {
      setMessage(t("plannedNote"));
      return false;
    }
    try {
      await api.updateUser(token, libraryAccessTarget.id, {
        libraryMemberships: libraryMembershipsFromRoles(roles)
      });
      setLibraryRoles(roles);
      setMessage(t("saved"));
      await refreshAll();
      return true;
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
      return false;
    }
  };

  const submitUser = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!editingUser && !email.trim()) {
      setMessage(t("formRequiredHint"));
      return;
    }
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      const libraryMemberships = libraryMembershipsFromRoles(libraryRoles);
      if (editingUser) {
        await api.updateUser(token, editingUser.id, {
          displayName: displayName.trim() || undefined,
          role,
          isActive,
          password: password.trim() || undefined,
          libraryMemberships
        });
      } else {
        await api.createUser(token, {
          email: email.trim(),
          password: defaultNewUserPassword,
          displayName: displayName.trim() || emailPrefix(email),
          role,
          libraryMemberships
        });
      }
      closeDialog();
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const toggleUserActive = async (user: TeamUser) => {
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      await api.updateUser(token, user.id, {
        displayName: user.displayName,
        role: user.globalRole,
        isActive: !user.isActive
      });
      setMessage(t("saved"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const openAvatarDialog = async (user: TeamUser) => {
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    setAvatarTarget(user);
    try {
      setAvatars(await api.listSystemAvatars(token));
    } catch {
      setAvatars(defaultSystemAvatars);
    }
  };

  const updateAvatar = async (avatarKey: string) => {
    if (!token || !avatarTarget) return;
    setAvatarBusy(true);
    try {
      await api.updateUserAvatar(token, avatarTarget.id, avatarKey);
      setAvatarTarget(null);
      setMessage(t("avatarUpdated"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setAvatarBusy(false);
    }
  };

  const deleteUser = async () => {
    if (!token || !deleteTarget) return;
    setDeleteBusy(true);
    try {
      await api.deleteUser(token, deleteTarget.id);
      setDeleteTarget(null);
      setMessage(t("userDeleted"));
      await refreshAll();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setDeleteBusy(false);
    }
  };

  return (
    <PageFrame
      title={t("users")}
      description={t("usersPageHint")}
      action={
        <Button type="button" onClick={openCreateDialog} disabled={!canManageUsers}>
          <UserPlus size={15} />
          <span>{t("createUser")}</span>
        </Button>
      }
    >
      <UserDialog
        canManageAccountIdentity={canManageAccountIdentity}
        displayName={displayName}
        editingUser={editingUser}
        email={email}
        isActive={isActive}
        libraries={libraries}
        libraryRoles={libraryRoles}
        open={dialogOpen}
        password={password}
        role={role}
        t={t}
        onClose={closeDialog}
        onDisplayNameChange={setDisplayName}
        onEmailChange={setEmail}
        onIsActiveChange={setIsActive}
        onLibraryAccessManageOpen={() => {
          setLibraryAccessTarget(null);
          setLibraryAccessManageOpen(true);
        }}
        onPasswordChange={setPassword}
        onRoleChange={setRole}
        onSubmit={submitUser}
      />
      <UserLibraryAccessDialog
        libraries={libraries}
        libraryRoles={libraryRoles}
        open={libraryAccessManageOpen}
        subject={libraryAccessTarget?.displayName ?? editingUser?.displayName}
        t={t}
        onClose={closeLibraryAccessDialog}
        onSave={saveLibraryAccess}
      />
      <AvatarDialog
        avatars={avatars}
        busy={avatarBusy}
        currentAvatarKey={avatarTarget?.avatarKey}
        open={Boolean(avatarTarget)}
        t={t}
        targetName={avatarTarget?.displayName ?? ""}
        onClose={() => !avatarBusy && setAvatarTarget(null)}
        onSelect={(avatarKey) => { void updateAvatar(avatarKey); }}
      />
      <DeleteUserDialog
        busy={deleteBusy}
        open={Boolean(deleteTarget)}
        t={t}
        user={deleteTarget}
        onClose={() => !deleteBusy && setDeleteTarget(null)}
        onConfirm={deleteUser}
      />
      <section className="management-list-card users-surface">
        <div className="users-toolbar">
          <label className="users-search">
            <Search size={15} />
            <input aria-label={t("searchUsers")} value={query} placeholder={t("searchUsers")} onChange={(event) => setQuery(event.target.value)} />
          </label>
          <StatusDot label={`${t("onlineUsers")} ${onlineUserCount}/${users.length}`} tone={onlineUserCount > 0 ? "good" : "muted"} />
        </div>
        <div className="table-wrap users-table-wrap">
          <table className="data-table users-table">
            <thead>
              <tr>
                <th className="users-identity-column">{t("users")}</th>
                <th className="users-libraries-column">{t("libraryAccess")}</th>
                <th className="users-activity-column">{t("recentActivity")}</th>
                <th className="users-status-column">{t("status")}</th>
                <th className="users-actions-column">{t("action")}</th>
              </tr>
            </thead>
            <tbody>
              {filteredUsers.length === 0 ? (
                <tr>
                  <td colSpan={5}>{t("noSearchResults")}</td>
                </tr>
              ) : filteredUsers.map((item) => {
                  const hasGlobalLibraryAccess = item.globalRole === "owner" || item.globalRole === "admin";
                  const canEditUser = canManageUsers && (canManageAccountIdentity || !hasGlobalLibraryAccess);
                  const canDeleteUser = canEditUser && currentUser?.id !== item.id;
                  const online = isUserOnline(item);
                  const activityTime = online ? item.lastSeenAt : item.lastLoginAt;
                  const activityLabel = online ? t("lastActive") : t("lastLogin");
                  return (
                    <tr key={item.id}>
                      <td className="users-identity-column">
                        <div className="user-identity-cell">
                          <UserAvatar
                            avatarKey={item.avatarKey}
                            label={item.displayName}
                            size="lg"
                            onClick={canEditUser ? () => { void openAvatarDialog(item); } : undefined}
                          />
                          <div className="user-identity-main">
                            <div className="user-identity-heading">
                              <strong>{item.displayName}</strong>
                              {(item.globalRole === "owner" || item.globalRole === "admin") && (
                                <em className={`is-${item.globalRole}`}>{roleLabel(t, item.globalRole)}</em>
                              )}
                            </div>
                            <span>{item.email}</span>
                          </div>
                        </div>
                      </td>
                      <td className="users-libraries-column">
                        <div className="user-library-access-cell">
                          <div
                            className="user-library-access"
                            title={hasGlobalLibraryAccess
                              ? t("serverRoleGrantsAllLibraries")
                              : (item.libraryMemberships ?? []).map((membership) => `${membership.libraryName}: ${roleLabel(t, membership.role)}`).join("\n")}
                          >
                            {hasGlobalLibraryAccess ? (
                              <span className="user-library-chip is-global">
                                <strong>{t("allLibraries")}</strong>
                              </span>
                            ) : (item.libraryMemberships ?? []).length === 0 ? (
                              null
                            ) : (
                              <>
                                {(item.libraryMemberships ?? []).slice(0, 2).map((membership) => (
                                  <span className="user-library-chip" key={membership.libraryId}>
                                    <strong>{membership.libraryName}</strong>
                                    <em>{roleLabel(t, membership.role)}</em>
                                  </span>
                                ))}
                                {(item.libraryMemberships ?? []).length > 2 && (
                                  <span className="user-library-more">+{(item.libraryMemberships ?? []).length - 2}</span>
                                )}
                              </>
                            )}
                          </div>
                          {!hasGlobalLibraryAccess && (
                            <Button
                              className="user-library-access-button"
                              disabled={!canEditUser || libraries.length === 0}
                              size="icon"
                              title={t("manageLibraryAccess")}
                              aria-label={t("manageLibraryAccess")}
                              type="button"
                              variant="ghost"
                              onClick={() => openLibraryAccessDialog(item)}
                            >
                              <Plus size={14} />
                            </Button>
                          )}
                        </div>
                      </td>
                      <td className="users-activity-column">
                        <div className="user-activity-state" title={`${activityLabel}: ${formatDateTime(activityTime)}`}>
                          <span className={`user-state${online ? " is-on" : " is-off"}`}>
                            <span aria-hidden="true" />
                            {online ? t("online") : t("offline")}
                          </span>
                          <time>{formatDateTime(activityTime)}</time>
                        </div>
                      </td>
                      <td className="users-status-column">
                        <span className={`user-state${item.isActive ? " is-on" : " is-off"}`}>
                          <span aria-hidden="true" />
                          {item.isActive ? t("enabled") : t("disabled")}
                        </span>
                      </td>
                      <td className="users-actions-column">
                        <div className="users-actions">
                          <Button className="users-action-button" size="icon" type="button" variant="ghost" title={t("edit")} aria-label={t("edit")} onClick={() => openEditDialog(item)} disabled={!canEditUser}>
                            <Pencil size={14} />
                          </Button>
                          <Button className="users-action-button" size="icon" type="button" variant="ghost" title={item.isActive ? t("deactivate") : t("activate")} aria-label={item.isActive ? t("deactivate") : t("activate")} onClick={() => void toggleUserActive(item)} disabled={!canEditUser}>
                            <Power size={14} />
                          </Button>
                          <Button className="users-action-button is-destructive" size="icon" type="button" variant="ghost" title={t("deleteUser")} aria-label={t("deleteUser")} onClick={() => setDeleteTarget(item)} disabled={!canDeleteUser}>
                            <Trash2 size={14} />
                          </Button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
            </tbody>
          </table>
        </div>
      </section>
    </PageFrame>
  );
}
