import type { FormEvent } from "react";
import { Button } from "@/components/ui/button";
import type { TeamUser } from "../../api";
import type { TranslatorContext } from "../../types";
import { TextField } from "../common";
import { DialogShell } from "./dialog-shell";

export function UserDialog({
  canAssignOwner,
  displayName,
  editingUser,
  email,
  isActive,
  open,
  password,
  role,
  t,
  onClose,
  onDisplayNameChange,
  onEmailChange,
  onIsActiveChange,
  onPasswordChange,
  onRoleChange,
  onSubmit
}: TranslatorContext & {
  canAssignOwner: boolean;
  displayName: string;
  editingUser: TeamUser | null;
  email: string;
  isActive: boolean;
  open: boolean;
  password: string;
  role: string;
  onClose: () => void;
  onDisplayNameChange: (value: string) => void;
  onEmailChange: (value: string) => void;
  onIsActiveChange: (value: boolean) => void;
  onPasswordChange: (value: string) => void;
  onRoleChange: (value: string) => void;
  onSubmit: (event: FormEvent) => void | Promise<void>;
}) {
  const title = editingUser ? t("editUser") : t("createUser");

  return (
    <DialogShell
      className="user-dialog"
      closeLabel={t("cancel")}
      open={open}
      subtitle={t("usersPageHint")}
      title={title}
      titleId="user-dialog-title"
      onClose={onClose}
    >
      <form className="dialog-form" onSubmit={onSubmit}>
        <div className="dialog-body">
          {editingUser ? (
            <div className="readonly-field">
              <span>{t("email")}</span>
              <strong>{email}</strong>
            </div>
          ) : (
            <TextField autoFocus required label={t("email")} value={email} onChange={onEmailChange} />
          )}
          <TextField label={t("displayName")} value={displayName} onChange={onDisplayNameChange} />
          <label className="field">
            <span>{t("role")}</span>
            <select value={role} onChange={(event) => onRoleChange(event.target.value)}>
              {canAssignOwner && <option value="owner">{t("owner")}</option>}
              <option value="admin">{t("adminRole")}</option>
              <option value="library_manager">{t("manager")}</option>
              <option value="editor">{t("editor")}</option>
              <option value="viewer">{t("viewer")}</option>
            </select>
          </label>
          {editingUser && (
            <label className="field">
              <span>{t("status")}</span>
              <select value={isActive ? "enabled" : "disabled"} onChange={(event) => onIsActiveChange(event.target.value === "enabled")}>
                <option value="enabled">{t("enabled")}</option>
                <option value="disabled">{t("disabled")}</option>
              </select>
            </label>
          )}
          <TextField
            label={editingUser ? t("newPassword") : t("password")}
            value={password}
            onChange={onPasswordChange}
            type="password"
            required={!editingUser}
            placeholder={editingUser ? t("passwordOptional") : undefined}
          />
          {editingUser && <p className="dialog-hint">{t("passwordOptional")}</p>}
        </div>
        <div className="dialog-footer">
          <Button type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
          <Button type="submit">{t("submit")}</Button>
        </div>
      </form>
    </DialogShell>
  );
}
