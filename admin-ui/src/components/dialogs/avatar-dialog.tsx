import { Button } from "@/components/ui/button";
import type { SystemAvatar } from "../../api";
import type { TranslatorContext } from "../../types";
import { UserAvatar } from "../common";
import { DialogShell } from "./dialog-shell";

export function AvatarDialog({
  avatars,
  busy,
  currentAvatarKey,
  open,
  t,
  targetName,
  onClose,
  onSelect
}: TranslatorContext & {
  avatars: SystemAvatar[];
  busy: boolean;
  currentAvatarKey?: string | null;
  open: boolean;
  targetName: string;
  onClose: () => void;
  onSelect: (avatarKey: string) => void;
}) {
  return (
    <DialogShell
      className="avatar-dialog"
      closeLabel={t("cancel")}
      open={open}
      subtitle={targetName}
      title={t("changeAvatar")}
      titleId="avatar-dialog-title"
      onClose={onClose}
    >
      <div className="avatar-picker-grid">
        {avatars.map((avatar) => (
          <button
            className={`avatar-picker-option${avatar.key === currentAvatarKey ? " is-selected" : ""}`}
            disabled={busy}
            key={avatar.key}
            title={avatar.key}
            type="button"
            onClick={() => onSelect(avatar.key)}
          >
            <UserAvatar avatarKey={avatar.key} label={avatar.key} size="lg" />
          </button>
        ))}
      </div>
      <div className="dialog-actions">
        <Button type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
      </div>
    </DialogShell>
  );
}
