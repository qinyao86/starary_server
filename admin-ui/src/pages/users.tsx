import { useMemo, useState } from "react";
import { Pencil, Power, Search, UserPlus, Users } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { TeamUser } from "../api";
import { api } from "../api";
import type { PageContext } from "../types";
import { PageFrame, Panel, StatusDot } from "../components/common";
import { UserDialog } from "../components/dialogs";
import { canManageServerRole, formatDateTime, isUserOnline, roleLabel } from "../utils/format";

export function UsersPage({ t, token, users, currentUser, refreshAll, setMessage }: PageContext) {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingUser, setEditingUser] = useState<TeamUser | null>(null);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [role, setRole] = useState("viewer");
  const [isActive, setIsActive] = useState(true);
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
        item.lastSeenLibraryName ?? ""
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
    setDialogOpen(true);
  };

  const openEditDialog = (user: TeamUser) => {
    setEditingUser(user);
    setEmail(user.email);
    setPassword("");
    setDisplayName(user.displayName);
    setRole(user.globalRole);
    setIsActive(user.isActive);
    setDialogOpen(true);
  };

  const closeDialog = () => {
    setDialogOpen(false);
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
      if (editingUser) {
        await api.updateUser(token, editingUser.id, {
          displayName: displayName.trim() || undefined,
          role,
          isActive,
          password: password.trim() || undefined
        });
      } else {
        await api.createUser(token, {
          email: email.trim(),
          password,
          displayName: displayName.trim() || undefined,
          role
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
        canAssignOwner={canAssignOwner}
        displayName={displayName}
        editingUser={editingUser}
        email={email}
        isActive={isActive}
        open={dialogOpen}
        password={password}
        role={role}
        t={t}
        onClose={closeDialog}
        onDisplayNameChange={setDisplayName}
        onEmailChange={setEmail}
        onIsActiveChange={setIsActive}
        onPasswordChange={setPassword}
        onRoleChange={setRole}
        onSubmit={submitUser}
      />
      <Panel
        title={t("teamAccess")}
        icon={Users}
        className="span-12"
      >
        <div className="toolbar-strip">
          <div className="search-box">
            <Search size={16} />
            <input value={query} placeholder={t("search")} onChange={(event) => setQuery(event.target.value)} />
          </div>
          <StatusDot label={`${t("onlineUsers")} ${onlineUserCount}/${users.length}`} tone={onlineUserCount > 0 ? "good" : "muted"} />
        </div>
        <div className="table-wrap">
          <table className="data-table">
            <thead>
              <tr>
                <th>{t("name")}</th>
                <th>{t("role")}</th>
                <th>{t("onlineStatus")}</th>
                <th>{t("lastLogin")}</th>
                <th>{t("recentLibrary")}</th>
                <th>{t("status")}</th>
                <th>{t("action")}</th>
              </tr>
            </thead>
            <tbody>
              {filteredUsers.length === 0 ? (
                <tr>
                  <td colSpan={7}>{t("empty")}</td>
                </tr>
              ) : (
                filteredUsers.map((item) => {
                  const isOwnerUser = item.globalRole === "owner";
                  const canEditUser = canManageUsers && (canAssignOwner || !isOwnerUser);
                  const online = isUserOnline(item);
                  return (
                    <tr key={item.id}>
                      <td>
                        <div className="user-identity-cell">
                          <strong>{item.displayName}</strong>
                          <span>{item.email}</span>
                        </div>
                      </td>
                      <td>{roleLabel(t, item.globalRole)}</td>
                      <td>
                        <div className="presence-cell">
                          <span className={`status-pill${online ? " is-on" : " is-off"}`}>{online ? t("online") : t("offline")}</span>
                          <span>{formatDateTime(item.lastSeenAt)}</span>
                        </div>
                      </td>
                      <td>{formatDateTime(item.lastLoginAt)}</td>
                      <td>{item.lastSeenLibraryName ?? "-"}</td>
                      <td><span className={`status-pill${item.isActive ? " is-on" : " is-off"}`}>{item.isActive ? t("enabled") : t("disabled")}</span></td>
                      <td>
                        <div className="table-actions">
                          <Button className="table-action-button" size="sm" type="button" variant="outline" onClick={() => openEditDialog(item)} disabled={!canEditUser}>
                            <Pencil size={14} />
                            <span>{t("edit")}</span>
                          </Button>
                          <Button className="table-action-button" size="sm" type="button" variant="outline" onClick={() => void toggleUserActive(item)} disabled={!canEditUser}>
                            <Power size={14} />
                            <span>{item.isActive ? t("deactivate") : t("activate")}</span>
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
      </Panel>
    </PageFrame>
  );
}
