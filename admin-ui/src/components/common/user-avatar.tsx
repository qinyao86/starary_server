import { useEffect, useState } from "react";
import { avatarUrl } from "../../utils/avatars";

export function UserAvatar({
  avatarKey,
  label,
  onClick,
  size = "md",
  updatedAt,
  userId
}: {
  avatarKey?: string | null;
  label: string;
  onClick?: () => void;
  size?: "sm" | "md" | "lg";
  updatedAt?: string;
  userId?: string;
}) {
  const [imageFailed, setImageFailed] = useState(false);
  const [useSystemFallback, setUseSystemFallback] = useState(false);
  const fallback = label.trim().charAt(0).toUpperCase() || "U";
  useEffect(() => {
    setImageFailed(false);
    setUseSystemFallback(false);
  }, [avatarKey, updatedAt, userId]);

  const isCustom = avatarKey?.startsWith("custom:") ?? false;
  const imageUrl = useSystemFallback
    ? avatarUrl(avatarKey)
    : avatarUrl(avatarKey, userId, updatedAt);

  const content = imageUrl && !imageFailed ? (
    <img
      alt=""
      src={imageUrl}
      onError={() => {
        if (isCustom && !useSystemFallback) setUseSystemFallback(true);
        else setImageFailed(true);
      }}
    />
  ) : (
    <span>{fallback}</span>
  );

  if (onClick) {
    return (
      <button
        aria-label={label}
        className={`user-avatar user-avatar-${size} is-clickable${isCustom ? " is-custom" : ""}`}
        title={label}
        type="button"
        onClick={onClick}
      >
        {content}
      </button>
    );
  }

  return (
    <span className={`user-avatar user-avatar-${size}${isCustom ? " is-custom" : ""}`} title={label}>
      {content}
    </span>
  );
}
