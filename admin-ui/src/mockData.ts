import type { TranslationKey } from "./i18n";

export const metricCards = [
  { key: "activeUsers", value: "18", change: "+3", tone: "blue" },
  { key: "teamLibraries", value: "4", change: "+1", tone: "green" },
  { key: "totalAssets", value: "128,430", change: "+6.2%", tone: "violet" },
  { key: "storageUsed", value: "8.7 TB", change: "+420 GB", tone: "amber" }
] as const satisfies ReadonlyArray<{
  key: TranslationKey;
  value: string;
  change: string;
  tone: "blue" | "green" | "violet" | "amber";
}>;

export const libraries = [
  { name: "Studio Assets", assets: "86,210", members: 24, storage: "5.8 TB", policy: "ownerOnly" },
  { name: "Lookdev Reference", assets: "21,004", members: 12, storage: "1.9 TB", policy: "editorsAll" },
  { name: "Scans Archive", assets: "18,980", members: 8, storage: "980 GB", policy: "managerOnly" },
  { name: "Training", assets: "2,236", members: 6, storage: "42 GB", policy: "readOnly" }
] as const;

export const users = [
  { name: "Qin Yao", email: "owner@madlibrary.local", role: "owner", status: "enabled", lastActive: "minutesAgo" },
  { name: "Lena Wu", email: "lena@studio.local", role: "manager", status: "enabled", lastActive: "eighteenMinutesAgo" },
  { name: "Marco Lee", email: "marco@studio.local", role: "editor", status: "enabled", lastActive: "oneHourAgo" },
  { name: "Iris Chen", email: "iris@studio.local", role: "viewer", status: "pending", lastActive: "invitePending" }
] as const;

export const sharedRoots = [
  {
    name: "Project NAS",
    provider: "SMB",
    canonicalUri: "smb://192.168.3.13/projects",
    windowsHint: "\\\\192.168.3.13\\projects",
    macosHint: "/Volumes/projects",
    mode: "reference"
  },
  {
    name: "Server Storage",
    provider: "Filesystem",
    canonicalUri: "server://storage/assets",
    windowsHint: "D:\\MadLibraryTeamStorage",
    macosHint: "N/A",
    mode: "copy"
  },
  {
    name: "Cloud Bucket",
    provider: "S3",
    canonicalUri: "s3://madlibrary-team-assets",
    windowsHint: "N/A",
    macosHint: "N/A",
    mode: "copy"
  }
];

export const activity = [
  { actor: "Lena Wu", action: "importedAssets", target: "Studio Assets", time: "10:42" },
  { actor: "Qin Yao", action: "registeredSharedRoot", target: "Project NAS", time: "09:18" },
  { actor: "Marco Lee", action: "updatedTags", target: "Moon Saber", time: "yesterday" },
  { actor: "system", action: "completedBackup", target: "Nightly", time: "yesterday" }
] as const;

export const permissions = [
  { action: "manageServer", owner: true, admin: true, manager: false, editor: false, viewer: false },
  { action: "manageUsers", owner: true, admin: true, manager: true, editor: false, viewer: false },
  { action: "manageLibrary", owner: true, admin: true, manager: true, editor: false, viewer: false },
  { action: "importAssets", owner: true, admin: true, manager: true, editor: true, viewer: false },
  { action: "editOwnAssets", owner: true, admin: true, manager: true, editor: true, viewer: false },
  { action: "editAllAssets", owner: true, admin: true, manager: true, editor: false, viewer: false },
  { action: "deleteAssets", owner: true, admin: true, manager: true, editor: false, viewer: false },
  { action: "manageTags", owner: true, admin: true, manager: true, editor: false, viewer: false },
  { action: "downloadAssets", owner: true, admin: true, manager: true, editor: true, viewer: true }
] as const;

export const assetBreakdown = [
  { label: "images", value: 38 },
  { label: "models3d", value: 26 },
  { label: "textures", value: 21 },
  { label: "video", value: 9 },
  { label: "docs", value: 6 }
] as const;
