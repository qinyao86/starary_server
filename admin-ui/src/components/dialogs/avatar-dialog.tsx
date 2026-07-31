import { Button } from "@/components/ui/button";
import { Upload } from "lucide-react";
import { useRef } from "react";
import type { SystemAvatar } from "../../api";
import type { TranslatorContext } from "../../types";
import { UserAvatar } from "../common";
import { DialogShell } from "./dialog-shell";

const AVATAR_AUTHOR_URL = "https://www.iconfont.cn/user/detail?spm=a313x.collections_detail.i1.d9bd4f23f.31a03a81lo3uYS&uid=14278&nid=Dl8y7W8raO8r";

export function AvatarDialog({
  avatars,
  busy,
  currentAvatarKey,
  currentAvatarUpdatedAt,
  currentAvatarUserId,
  open,
  t,
  targetName,
  onClose,
  onSelect,
  onUpload
}: TranslatorContext & {
  avatars: SystemAvatar[];
  busy: boolean;
  currentAvatarKey?: string | null;
  currentAvatarUpdatedAt?: string;
  currentAvatarUserId?: string;
  open: boolean;
  targetName: string;
  onClose: () => void;
  onSelect: (avatarKey: string) => void;
  onUpload: (file: File) => void | Promise<void>;
}) {
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  return (
    <DialogShell
      className="avatar-dialog"
      closeLabel={t("cancel")}
      open={open}
      subtitle={targetName}
      title={t("changeAvatar")}
      titleId="avatar-dialog-title"
      onClose={onClose}
      headerContent={
        <div className="avatar-dialog-identity">
          <button
            aria-label={t("uploadAvatar")}
            className="avatar-custom-trigger"
            disabled={busy}
            type="button"
            onClick={() => fileInputRef.current?.click()}
          >
            <UserAvatar
              avatarKey={currentAvatarKey}
              label={targetName}
              size="lg"
              updatedAt={currentAvatarUpdatedAt}
              userId={currentAvatarUserId}
            />
            <span className="avatar-custom-trigger-icon" title={t("uploadAvatar")}><Upload size={15} /></span>
          </button>
          <div>
            <h2 className="dialog-title" id="avatar-dialog-title">{t("changeAvatar")}</h2>
            <p className="dialog-subtitle">{targetName}</p>
          </div>
        </div>
      }
    >
      <input
        accept="image/*"
        className="avatar-picker-file-input"
        ref={fileInputRef}
        type="file"
        onChange={(event) => {
          const file = event.target.files?.[0];
          event.target.value = "";
          if (file) void onUpload(file);
        }}
      />
      <div className="avatar-system-heading">{t("systemAvatar")}</div>
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
        <a
          className="avatar-author-link"
          href={AVATAR_AUTHOR_URL}
          rel="noreferrer"
          target="_blank"
        >
          {t("avatarAuthorCredit")}
        </a>
        <span className="avatar-dialog-action-spacer" />
        <Button type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
      </div>
    </DialogShell>
  );
}
