import type { ServerInfo, TeamUser } from "../api";
import { createTranslator } from "../i18n";
import type { DeploymentMode } from "../types";

export const userOnlineWindowMs = 2 * 60 * 1000;

export function formatCount(value?: number | null) {
  const count = value ?? 0;
  return count === 0 ? "-" : new Intl.NumberFormat().format(count);
}

export function formatBytes(value?: number | null) {
  const bytes = value ?? 0;
  if (bytes <= 0) return "-";
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  const unitIndex = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const amount = bytes / 1024 ** unitIndex;
  return `${amount >= 10 || unitIndex === 0 ? amount.toFixed(0) : amount.toFixed(1)} ${units[unitIndex]}`;
}

export function parseStorageSize(value: string) {
  const match = value.trim().match(/^([\d.]+)\s*([KMGTPE]?B)$/i);
  if (!match) return 0;
  const amount = Number(match[1]);
  const unit = match[2].toUpperCase();
  const units = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];
  const unitIndex = units.indexOf(unit);
  return Number.isFinite(amount) && unitIndex >= 0 ? Math.round(amount * 1024 ** unitIndex) : 0;
}

export function formatMemberNames(memberNames: string[] | undefined, fallback: string) {
  if (!memberNames || memberNames.length === 0) return fallback;
  if (memberNames.length <= 2) return memberNames.join(", ");
  return `${memberNames.slice(0, 2).join(", ")} +${memberNames.length - 2}`;
}

export function isUserOnline(user: TeamUser, now = Date.now()) {
  if (!user.isActive || !user.lastSeenAt) return false;
  const lastSeen = Date.parse(user.lastSeenAt);
  return Number.isFinite(lastSeen) && now - lastSeen <= userOnlineWindowMs;
}

export function formatDateTime(value?: string | null) {
  if (!value) return "-";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "-";
  return new Intl.DateTimeFormat(undefined, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit"
  }).format(date);
}

export function roleLabel(t: ReturnType<typeof createTranslator>, role: string) {
  if (role === "owner") return t("owner");
  if (role === "admin" || role === "adminRole") return t("adminRole");
  if (role === "library_manager" || role === "manager") return t("manager");
  if (role === "editor") return t("editor");
  if (role === "viewer") return t("viewer");
  return role;
}

export function isLibraryManagerRole(role: string) {
  return role === "owner" || role === "admin" || role === "library_manager" || role === "manager";
}

export function canManageServerRole(role: string) {
  return role === "owner" || role === "admin";
}

export function activityActionLabel(t: ReturnType<typeof createTranslator>, action: string) {
  switch (action) {
    case "server.owner_created":
      return t("ownerCreated");
    case "library.created":
      return t("libraryCreated");
    case "library.updated":
      return t("libraryUpdated");
    case "library.deleted":
      return t("libraryDeleted");
    case "library.enabled":
      return t("libraryEnabled");
    case "library.disabled":
      return t("libraryDisabled");
    case "library.member_upserted":
      return t("memberUpdated");
    case "library.member_removed":
      return t("memberRemoved");
    case "user.created":
      return t("userCreated");
    case "user.updated":
      return t("userUpdated");
    case "storage_root.created":
      return t("storageRootCreated");
    case "storage_root.updated":
      return t("storageRootUpdated");
    case "storage_root.deleted":
      return t("storageRootDeleted");
    default:
      return action;
  }
}

export function deploymentModeLabel(t: ReturnType<typeof createTranslator>, mode: DeploymentMode) {
  return mode === "local" ? t("local") : t("cloud");
}

export function storageStatusLabel(t: ReturnType<typeof createTranslator>, serverInfo: ServerInfo | null) {
  if (!serverInfo) return "-";
  if (serverInfo.storageStatus === "writable") return t("storageWritable");
  if (serverInfo.storageStatus === "read_only") return t("storageReadOnly");
  return t("storageMissing");
}

export function storageKindLabel(t: ReturnType<typeof createTranslator>, kind: string) {
  if (kind === "server_filesystem") return t("storageKindServerFilesystem");
  if (kind === "smb") return t("storageKindSmb");
  if (kind === "s3") return t("storageKindS3");
  return kind;
}

export function splitList(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}
