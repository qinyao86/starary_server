import { Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { TeamUser } from "../../api";
import type { TranslatorContext } from "../../types";
import { UserAvatar } from "../common/user-avatar";
import { DialogShell } from "./dialog-shell";

export function DeleteUserDialog({
  busy,
  open,
  t,
  user,
  onClose,
  onConfirm
}: TranslatorContext & {
  busy: boolean;
  open: boolean;
  user: TeamUser | null;
  onClose: () => void;
  onConfirm: () => void | Promise<void>;
}) {
  return (
    <DialogShell
      className="delete-user-dialog"
      closeLabel={t("cancel")}
      open={open}
      subtitle={t("deleteUserDialogHint")}
      title={t("deleteUserConfirm")}
      titleId="delete-user-dialog-title"
      onClose={onClose}
    >
      <div className="delete-user-body">
        <div className="delete-user-identity">
          <UserAvatar avatarKey={user?.avatarKey} label={user?.displayName ?? ""} size="lg" />
          <div>
            <strong>{user?.displayName ?? ""}</strong>
            <span>{user?.email ?? ""}</span>
          </div>
        </div>
        <p>{t("deleteUserDataHint")}</p>
      </div>
      <div className="dialog-footer">
        <Button disabled={busy} type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
        <Button disabled={busy} type="button" variant="destructive" onClick={() => void onConfirm()}>
          <Trash2 size={15} />
          {t("deleteUser")}
        </Button>
      </div>
    </DialogShell>
  );
}
