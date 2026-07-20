import type { SystemAvatar } from "../api";

export function avatarUrl(
  avatarKey?: string | null,
  userId?: string,
  updatedAt?: string
) {
  if (!avatarKey) return "";
  if (avatarKey.startsWith("custom:") && userId) {
    const params = updatedAt ? `?v=${encodeURIComponent(updatedAt)}` : "";
    return `/api/v1/avatars/users/${encodeURIComponent(userId)}${params}`;
  }
  return `/api/v1/avatars/system/${encodeURIComponent(avatarKey.replace(/^custom:/, ""))}`;
}

export const defaultSystemAvatars: SystemAvatar[] = [
  ...Array.from({ length: 20 }, (_, index) => {
    const key = `male-${String(index + 1).padStart(2, "0")}`;
    return { key, gender: "male" as const, url: avatarUrl(key) };
  }),
  ...Array.from({ length: 20 }, (_, index) => {
    const key = `female-${String(index + 1).padStart(2, "0")}`;
    return { key, gender: "female" as const, url: avatarUrl(key) };
  })
];
