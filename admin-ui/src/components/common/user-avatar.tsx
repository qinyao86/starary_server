import { useEffect, useState } from "react";
import { avatarUrl } from "../../utils/avatars";

export function UserAvatar({
  avatarKey,
  label,
  onClick,
  size = "md"
}: {
  avatarKey?: string | null;
  label: string;
  onClick?: () => void;
  size?: "sm" | "md" | "lg";
}) {
  const [imageFailed, setImageFailed] = useState(false);
  const fallback = label.trim().charAt(0).toUpperCase() || "U";
  useEffect(() => {
    setImageFailed(false);
  }, [avatarKey]);

  const content = avatarKey && !imageFailed ? (
    <img alt="" src={avatarUrl(avatarKey)} onError={() => setImageFailed(true)} />
  ) : (
    <span>{fallback}</span>
  );

  if (onClick) {
    return (
      <button
        aria-label={label}
        className={`user-avatar user-avatar-${size} is-clickable`}
        title={label}
        type="button"
        onClick={onClick}
      >
        {content}
      </button>
    );
  }

  return (
    <span className={`user-avatar user-avatar-${size}`} title={label}>
      {content}
    </span>
  );
}
