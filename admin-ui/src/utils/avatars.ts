import type { SystemAvatar } from "../api";

export function avatarUrl(avatarKey?: string | null) {
  return avatarKey ? `/api/v1/avatars/system/${encodeURIComponent(avatarKey)}` : "";
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
