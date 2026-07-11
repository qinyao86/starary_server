import type { FormEvent } from "react";
import { Button } from "@/components/ui/button";
import type { TeamUser } from "../../api";
import type { TranslatorContext } from "../../types";
import { SelectField } from "../common";
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
              <SelectField label={t("users")} value={memberUserId} onChange={onUserChange}>
                  {users.map((item) => (
                    <option key={item.id} value={item.id}>{item.displayName} ({item.email})</option>
                  ))}
              </SelectField>
              <SelectField label={t("role")} value={memberRole} onChange={onRoleChange}>
                  <option value="library_manager">{t("manager")}</option>
                  <option value="editor">{t("editor")}</option>
                  <option value="viewer">{t("viewer")}</option>
              </SelectField>
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
