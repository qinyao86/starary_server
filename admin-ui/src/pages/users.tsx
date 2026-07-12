import { useMemo, useState } from "react";
import { Pencil, Power, Search, UserPlus } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { TeamUser } from "../api";
import { api } from "../api";
import type { PageContext } from "../types";
import { PageFrame, StatusDot } from "../components/common";
import { UserDialog, UserLibraryAccessDialog } from "../components/dialogs";
import { canManageServerRole, formatDateTime, isUserOnline, roleLabel } from "../utils/format";

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
  const [query, setQuery] = useState("");

  const canManageUsers = canManageServerRole(currentUser?.role ?? "");
  const canAssignOwner = currentUser?.role === "owner";
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
    setRole(user.globalRole);
    setIsActive(user.isActive);
    setLibraryRoles(Object.fromEntries((user.libraryMemberships ?? []).map((membership) => [membership.libraryId, membership.role])));
    setDialogOpen(true);
  };

  const closeDialog = () => {
    setDialogOpen(false);
    setLibraryAccessManageOpen(false);
  };

  const submitUser = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!editingUser && (!email.trim() || !password.trim())) {
      setMessage(t("formRequiredHint"));
      return;
    }
    if (!token) {
      setMessage(t("plannedNote"));
      return;
    }
    try {
      const libraryMemberships = libraries.flatMap((library) => {
        const membershipRole = libraryRoles[library.id] ?? "";
        return membershipRole ? [{ libraryId: library.id, role: membershipRole }] : [];
      });
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
          password,
          displayName: displayName.trim() || undefined,
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
        displayName={displayName}
        editingUser={editingUser}
        email={email}
        isActive={isActive}
        libraries={libraries}
        libraryRoles={libraryRoles}
        open={dialogOpen}
        password={password}
        t={t}
        onClose={closeDialog}
        onDisplayNameChange={setDisplayName}
        onEmailChange={setEmail}
        onIsActiveChange={setIsActive}
        onLibraryAccessManageOpen={() => setLibraryAccessManageOpen(true)}
        onPasswordChange={setPassword}
        onSubmit={submitUser}
      />
      <UserLibraryAccessDialog
        libraries={libraries}
        libraryRoles={libraryRoles}
        open={libraryAccessManageOpen}
        t={t}
        onClose={() => setLibraryAccessManageOpen(false)}
        onSave={setLibraryRoles}
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
                <th className="users-activity-column">{t("lastActive")}</th>
                <th className="users-login-column">{t("lastLogin")}</th>
                <th className="users-status-column">{t("status")}</th>
                <th className="users-actions-column">{t("action")}</th>
              </tr>
            </thead>
            <tbody>
              {filteredUsers.length === 0 ? (
                <tr>
                  <td colSpan={6}>{t("empty")}</td>
                </tr>
              ) : (
                filteredUsers.map((item) => {
                  const isOwnerUser = item.globalRole === "owner";
                  const canEditUser = canManageUsers && (canAssignOwner || !isOwnerUser);
                  const online = isUserOnline(item);
                  return (
                    <tr key={item.id}>
                      <td className="users-identity-column">
                        <div className="user-identity-cell">
                          <div className="user-identity-heading">
                            <strong>{item.displayName}</strong>
                            <em>{roleLabel(t, item.globalRole)}</em>
                          </div>
                          <span>{item.email}</span>
                        </div>
                      </td>
                      <td className="users-libraries-column">
                        <div
                          className="user-library-access"
                          title={(item.globalRole === "owner" || item.globalRole === "admin")
                            ? t("serverRoleGrantsAllLibraries")
                            : (item.libraryMemberships ?? []).map((membership) => `${membership.libraryName}: ${roleLabel(t, membership.role)}`).join("\n")}
                        >
                          {item.globalRole === "owner" || item.globalRole === "admin" ? (
                            <span className="user-library-chip is-global">
                              <strong>{t("allLibraries")}</strong>
                              <em>{roleLabel(t, item.globalRole)}</em>
                            </span>
                          ) : (item.libraryMemberships ?? []).length === 0 ? (
                            <span className="user-library-empty">{t("noLibraryAccess")}</span>
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
                      </td>
                      <td className="users-activity-column">
                        <div className="user-activity-state" title={formatDateTime(item.lastSeenAt)}>
                          <span className={`user-state${online ? " is-on" : " is-off"}`}>
                            <span aria-hidden="true" />
                            {online ? t("online") : t("offline")}
                          </span>
                          <time>{formatDateTime(item.lastSeenAt)}</time>
                        </div>
                      </td>
                      <td className="users-login-column">{formatDateTime(item.lastLoginAt)}</td>
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
                        </div>
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </section>
    </PageFrame>
  );
}
