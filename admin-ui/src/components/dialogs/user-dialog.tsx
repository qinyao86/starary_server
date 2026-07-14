import { useMemo } from "react";
import type { FormEvent } from "react";
import { Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { TeamLibrary, TeamUser } from "../../api";
import { defaultNewUserPassword } from "../../constants";
import type { TranslatorContext } from "../../types";
import { SelectField, TextField } from "../common";
import { roleLabel } from "../../utils/format";
import { DialogShell } from "./dialog-shell";

export function UserDialog({
  displayName,
  editingUser,
  email,
  canManageAccountIdentity,
  isActive,
  libraries,
  libraryRoles,
  open,
  password,
  role,
  t,
  onClose,
  onDisplayNameChange,
  onEmailChange,
  onIsActiveChange,
  onLibraryAccessManageOpen,
  onPasswordChange,
  onRoleChange,
  onSubmit
}: TranslatorContext & {
  canManageAccountIdentity: boolean;
  displayName: string;
  editingUser: TeamUser | null;
  email: string;
  isActive: boolean;
  libraries: TeamLibrary[];
  libraryRoles: Record<string, string>;
  open: boolean;
  password: string;
  role: string;
  onClose: () => void;
  onDisplayNameChange: (value: string) => void;
  onEmailChange: (value: string) => void;
  onIsActiveChange: (value: boolean) => void;
  onLibraryAccessManageOpen: () => void;
  onPasswordChange: (value: string) => void;
  onRoleChange: (value: string) => void;
  onSubmit: (event: FormEvent) => void | Promise<void>;
}) {
  const title = editingUser ? t("editUser") : t("createUser");
  const hasGlobalLibraryAccess = role === "owner" || role === "admin";
  const accountIdentityLabel = role === "owner"
    ? t("owner")
    : role === "admin"
      ? t("adminRole")
      : t("standardUser");
  const assignedLibraries = useMemo(
    () => libraries.filter((library) => Boolean(libraryRoles[library.id])),
    [libraries, libraryRoles]
  );
  return (
    <DialogShell
      className="user-dialog"
      closeLabel={t("cancel")}
      open={open}
      subtitle={editingUser
        ? t("userDialogHint")
        : t("createUserDialogHint").replace("{password}", defaultNewUserPassword)}
      title={title}
      titleId="user-dialog-title"
      onClose={onClose}
    >
      <form autoComplete="off" className="dialog-form" onSubmit={onSubmit}>
        <div className="dialog-body">
          <div className="user-account-fields">
            {editingUser ? (
              <div className="readonly-field">
                <span>{t("email")}</span>
                <strong>{email}</strong>
              </div>
            ) : (
              <TextField
                autoComplete="off"
                autoFocus
                required
                label={t("email")}
                name="new-user-email"
                value={email}
                onChange={onEmailChange}
              />
            )}
            <TextField
              autoComplete="off"
              label={t("displayName")}
              name={editingUser ? "user-display-name" : "new-user-display-name"}
              value={displayName}
              onChange={onDisplayNameChange}
            />
            {canManageAccountIdentity && role !== "owner" ? (
              <SelectField label={t("accountIdentity")} value={role} onChange={onRoleChange}>
                <option value="viewer">{t("standardUser")}</option>
                <option value="admin">{t("adminRole")}</option>
              </SelectField>
            ) : (
              <div className="readonly-field">
                <span>{t("accountIdentity")}</span>
                <strong>{accountIdentityLabel}</strong>
              </div>
            )}
            {editingUser && (
              <TextField
                autoComplete="new-password"
                label={t("newPassword")}
                name="user-new-password"
                value={password}
                onChange={onPasswordChange}
                type="password"
                placeholder={t("passwordOptional")}
              />
            )}
            {editingUser && (
              <SelectField
                label={t("status")}
                value={isActive ? "enabled" : "disabled"}
                onChange={(value) => onIsActiveChange(value === "enabled")}
              >
                  <option value="enabled">{t("enabled")}</option>
                  <option value="disabled">{t("disabled")}</option>
              </SelectField>
            )}
          </div>
          <section className="user-library-editor">
            <div className="user-library-editor-topline">
              <div className="user-library-editor-heading">
                <strong>{t("libraryAccess")}</strong>
              </div>
              {!hasGlobalLibraryAccess && (
                <Button
                  className="user-library-manage-button"
                  disabled={libraries.length === 0}
                  size="icon"
                  title={t("manageLibraryAccess")}
                  aria-label={t("manageLibraryAccess")}
                  type="button"
                  variant="outline"
                  onClick={onLibraryAccessManageOpen}
                >
                  <Plus size={15} />
                </Button>
              )}
            </div>
            {hasGlobalLibraryAccess ? (
              <div className="user-library-inherited">{t("serverRoleGrantsAllLibraries")}</div>
            ) : libraries.length === 0 ? (
              <div className="user-library-inherited">{t("noLibraries")}</div>
            ) : (
              <div className="user-library-tags-editor">
                {assignedLibraries.length > 0 ? (
                  <div className="user-library-tags">
                    {assignedLibraries.map((library) => {
                      const membershipRole = libraryRoles[library.id];
                      return (
                        <div className="user-library-access-tag" key={library.id}>
                          <strong title={library.displayName}>{library.displayName}</strong>
                          <span className="user-library-tag-label">{roleLabel(t, membershipRole)}</span>
                        </div>
                      );
                    })}
                  </div>
                ) : (
                  <div className="user-library-inherited">{t("noLibraryAccess")}</div>
                )}
              </div>
            )}
          </section>
        </div>
        <div className="dialog-footer">
          <Button type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
          <Button type="submit">{t("submit")}</Button>
        </div>
      </form>
    </DialogShell>
  );
}
