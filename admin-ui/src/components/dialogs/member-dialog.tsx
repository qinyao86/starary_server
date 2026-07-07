import type { FormEvent } from "react";
import { Button } from "@/components/ui/button";
import type { TeamUser } from "../../api";
import type { TranslatorContext } from "../../types";
import { DialogShell } from "./dialog-shell";

export function MemberDialog({
  open,
  t,
  users,
  memberUserId,
  memberRole,
  onClose,
  onRoleChange,
  onSubmit,
  onUserChange
}: TranslatorContext & {
  open: boolean;
  users: TeamUser[];
  memberUserId: string;
  memberRole: string;
  onClose: () => void;
  onRoleChange: (value: string) => void;
  onSubmit: (event: FormEvent) => void | Promise<void>;
  onUserChange: (value: string) => void;
}) {
  return (
    <DialogShell
      className="member-dialog"
      closeLabel={t("cancel")}
      open={open}
      subtitle={t("addMemberHint")}
      title={t("addMember")}
      titleId="member-dialog-title"
      onClose={onClose}
    >
      <form className="dialog-form" onSubmit={onSubmit}>
        <div className="dialog-body">
          {users.length === 0 ? (
            <div className="placeholder-box">{t("noAvailableUsers")}</div>
          ) : (
            <>
              <label className="field">
                <span>{t("users")}</span>
                <select value={memberUserId} onChange={(event) => onUserChange(event.target.value)}>
                  {users.map((item) => (
                    <option key={item.id} value={item.id}>{item.displayName} ({item.email})</option>
                  ))}
                </select>
              </label>
              <label className="field">
                <span>{t("role")}</span>
                <select value={memberRole} onChange={(event) => onRoleChange(event.target.value)}>
                  <option value="library_manager">{t("manager")}</option>
                  <option value="editor">{t("editor")}</option>
                  <option value="viewer">{t("viewer")}</option>
                </select>
              </label>
            </>
          )}
        </div>
        <div className="dialog-footer">
          <Button type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
          <Button type="submit" disabled={!memberUserId || users.length === 0}>{t("submit")}</Button>
        </div>
      </form>
    </DialogShell>
  );
}
