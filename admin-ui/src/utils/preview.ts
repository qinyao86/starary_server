import type { CurrentUser, TeamLibrary, TeamUser } from "../api";
import { libraries as designLibraries, users as designUsers } from "../mockData";
import type { TranslatorContext } from "../types";
import { parseStorageSize } from "./format";

export function buildPreviewLibraries(t: TranslatorContext["t"]): TeamLibrary[] {
  return designLibraries.map((item, index) => ({
    id: `preview-library-${index}`,
    displayName: item.name,
    currentUserRole: index === 0 ? "owner" : "library_manager",
    accessMode: index === 0 ? "public" : "invite",
    isMember: true,
    enabled: index !== 2,
    libraryManagerNames: [designUsers[index % designUsers.length]?.name ?? "Qin Yao"],
    libraryManagerAvatarKeys: [`${index % 2 === 0 ? "male" : "female"}-${String((index % 20) + 1).padStart(2, "0")}`],
    memberNames: designUsers.slice(0, Math.min(3, item.members)).map((user) => user.name),
    assetCount: Number(item.assets.replace(/,/g, "")),
    folderCount: Math.max(0, Math.round(Number(item.assets.replace(/,/g, "")) / 120)),
    tagCount: Math.max(0, Math.round(Number(item.assets.replace(/,/g, "")) / 80)),
    totalSizeBytes: parseStorageSize(item.storage),
    storageRootCount: 1,
    enabledStorageRootCount: index === 2 ? 0 : 1,
    primaryStorageKind: "smb",
    primaryStorageUri: `smb://192.168.3.13/libraries/${item.name.toLowerCase().replace(/\s+/g, "-")}`,
    primaryStorageWindowsPath: `\\\\192.168.3.13\\libraries\\${item.name.toLowerCase().replace(/\s+/g, "-")}`,
    primaryStorageMacosPath: `smb://192.168.3.13/libraries/${item.name.toLowerCase().replace(/\s+/g, "-")}`
  }));
}

export function buildPreviewUsers(): TeamUser[] {
  const now = Date.now();
  return designUsers.map((item, index) => ({
    id: `preview-user-${index}`,
    email: item.email,
    displayName: item.name,
    avatarKey: `${index < 2 ? "male" : "female"}-${String((index % 20) + 1).padStart(2, "0")}`,
    globalRole: item.role === "manager" ? "library_manager" : item.role,
    isActive: item.status !== "pending",
    lastLoginAt: new Date(now - index * 30 * 60 * 1000).toISOString(),
    lastSeenAt: index < 2 ? new Date(now - index * 45 * 1000).toISOString() : new Date(now - index * 30 * 60 * 1000).toISOString(),
    lastSeenLibraryId: `preview-library-${index % designLibraries.length}`,
    lastSeenLibraryName: designLibraries[index % designLibraries.length]?.name ?? null,
    createdAt: new Date(now).toISOString(),
    updatedAt: new Date(now).toISOString()
  }));
}

export const previewCurrentUser: CurrentUser = {
  id: "preview-owner",
  email: "owner@starary.local",
  displayName: "Qin Yao",
  avatarKey: "male-01",
  role: "owner"
};
