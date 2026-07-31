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
  { name: "Qin Yao", email: "owner@starary.local", role: "owner", status: "enabled", lastActive: "minutesAgo" },
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
    windowsHint: "D:\\StararyTeamStorage",
    macosHint: "N/A",
    mode: "copy"
  },
  {
    name: "Cloud Bucket",
    provider: "S3",
    canonicalUri: "s3://starary-team-assets",
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

export const serverPermissions = [
  { action: "connectDesktopClient", owner: true, admin: true, user: true },
  { action: "loginConsole", owner: true, admin: true, user: "libraryRole" },
  { action: "manageOwnProfile", owner: true, admin: true, user: true },
  { action: "createLibrary", owner: true, admin: true, user: false },
  { action: "manageAllLibraries", owner: true, admin: true, user: false },
  { action: "manageStorage", owner: true, admin: true, user: false },
  { action: "manageUsers", owner: true, admin: true, user: false },
  { action: "viewServerStatistics", owner: true, admin: true, user: false },
  { action: "reviewPermissions", owner: true, admin: true, user: false },
  { action: "manageBackups", owner: true, admin: false, user: false },
  { action: "manageRuntime", owner: true, admin: true, user: false },
  { action: "initializeServer", owner: true, admin: false, user: false }
] as const;

type LibraryPermissionRow = {
  action: TranslationKey;
  category: TranslationKey;
  editor: boolean;
  manager: boolean;
  scope: "all" | "create" | "own" | "personal";
  target?: TranslationKey;
  verified: boolean;
  viewer: boolean;
};

function scopedRows(
  category: TranslationKey,
  action: TranslationKey,
  ownTarget: TranslationKey,
  allTarget: TranslationKey,
  verified: boolean,
): LibraryPermissionRow[] {
  return [
    { category, action, scope: "own", target: ownTarget, manager: true, editor: true, viewer: false, verified },
    { category, action, scope: "all", target: allTarget, manager: true, editor: false, viewer: false, verified },
  ];
}

export const libraryPermissions = [
  { category: "permissionCategoryAccess", action: "viewLibrary", scope: "all", manager: true, editor: true, viewer: true, verified: true },
  { category: "permissionCategoryAccess", action: "browseLibraryContent", scope: "all", manager: true, editor: true, viewer: true, verified: true },
  { category: "permissionCategoryAccess", action: "previewAssets", scope: "all", manager: true, editor: true, viewer: true, verified: true },
  { category: "permissionCategoryAccess", action: "downloadAssets", scope: "all", manager: true, editor: true, viewer: true, verified: true },
  { category: "permissionCategoryAccess", action: "copyAssetReferences", scope: "all", manager: true, editor: true, viewer: true, verified: true },
  { category: "permissionCategoryPersonal", action: "favoriteAssets", scope: "personal", manager: true, editor: true, viewer: true, verified: true },
  { category: "permissionCategoryPersonal", action: "managePersonalQuickAccess", scope: "personal", manager: true, editor: true, viewer: true, verified: true },
  { category: "permissionCategoryPersonal", action: "managePersonalFilterPresets", scope: "personal", manager: true, editor: true, viewer: true, verified: true },
  { category: "permissionCategoryImportTasks", action: "importAssets", scope: "create", manager: true, editor: true, viewer: false, verified: true },
  { category: "permissionCategoryImportTasks", action: "importImageSequences", scope: "create", manager: true, editor: true, viewer: false, verified: true },
  { category: "permissionCategoryImportTasks", action: "useSmartImportRules", scope: "create", manager: true, editor: true, viewer: false, verified: true },
  { category: "permissionCategoryImportTasks", action: "viewOwnTasks", scope: "personal", manager: true, editor: true, viewer: true, verified: false },
  { category: "permissionCategoryImportTasks", action: "controlOwnTasks", scope: "personal", manager: true, editor: true, viewer: true, verified: false },
  ...scopedRows("permissionCategoryAssetDetails", "renameAssets", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryAssetDetails", "editAssetDescriptions", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryAssetDetails", "editAssetLinks", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryAssetDetails", "rateAssets", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryAssetDetails", "editAssetViewerSettings", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryAssetDetails", "editTextAssetContent", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryAssetDetails", "convertAssetImportMode", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryAssetOrganization", "rebuildAssetThumbnails", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryAssetOrganization", "setCustomAssetThumbnails", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryAssetOrganization", "editImageSequences", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryAssetOrganization", "addAssetsToFolders", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryAssetOrganization", "removeAssetsFromFolders", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryAssetOrganization", "moveAssetFolders", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryAssetOrganization", "clearAssetFolders", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryAssetOrganization", "addAssetTags", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryAssetOrganization", "removeAssetTags", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryAssetOrganization", "replaceAssetTags", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryTrashDuplicates", "moveAssetsToTrash", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  { category: "permissionCategoryTrashDuplicates", action: "viewTrashAssets", scope: "own", target: "permissionTargetOwnAssets", manager: true, editor: true, viewer: true, verified: true },
  { category: "permissionCategoryTrashDuplicates", action: "viewTrashAssets", scope: "all", target: "permissionTargetAllAssets", manager: true, editor: false, viewer: false, verified: true },
  ...scopedRows("permissionCategoryTrashDuplicates", "restoreAssets", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryTrashDuplicates", "deleteAssetsPermanently", "permissionTargetOwnAssets", "permissionTargetAllAssets", true),
  ...scopedRows("permissionCategoryTrashDuplicates", "emptyTrash", "permissionTargetOwnAssets", "permissionTargetAllAssets", false),
  { category: "permissionCategoryTrashDuplicates", action: "viewDuplicateAssets", scope: "all", manager: true, editor: true, viewer: true, verified: false },
  { category: "permissionCategoryTrashDuplicates", action: "mergeDuplicateAssets", scope: "all", manager: true, editor: false, viewer: false, verified: true },
  { category: "permissionCategoryTransfer", action: "copyAssetsAcrossLibraries", scope: "own", target: "permissionTargetOwnAssets", manager: true, editor: true, viewer: false, verified: true },
  { category: "permissionCategoryTransfer", action: "copyAssetsAcrossLibraries", scope: "all", target: "permissionTargetAllAssets", manager: true, editor: false, viewer: false, verified: true },
  { category: "permissionCategoryTransfer", action: "copyFoldersAcrossLibraries", scope: "own", target: "permissionTargetOwnAssets", manager: true, editor: true, viewer: false, verified: true },
  { category: "permissionCategoryTransfer", action: "copyFoldersAcrossLibraries", scope: "all", target: "permissionTargetAllAssets", manager: true, editor: false, viewer: false, verified: true },
  { category: "permissionCategoryTransfer", action: "copyBetweenPersonalAndTeam", scope: "own", target: "permissionTargetOwnAssets", manager: true, editor: true, viewer: false, verified: true },
  { category: "permissionCategoryTransfer", action: "copyBetweenPersonalAndTeam", scope: "all", target: "permissionTargetAllAssets", manager: true, editor: false, viewer: false, verified: true },
  { category: "permissionCategoryTransfer", action: "exportAssets", scope: "all", manager: true, editor: true, viewer: true, verified: false },
  { category: "permissionCategoryFolders", action: "createRootFolders", scope: "create", manager: true, editor: true, viewer: false, verified: true },
  { category: "permissionCategoryFolders", action: "createChildFolders", scope: "own", target: "permissionTargetOwnFolders", manager: true, editor: true, viewer: false, verified: true },
  { category: "permissionCategoryFolders", action: "createChildFolders", scope: "all", target: "permissionTargetAllFolders", manager: true, editor: false, viewer: false, verified: true },
  ...scopedRows("permissionCategoryFolders", "renameFolders", "permissionTargetOwnFolders", "permissionTargetAllFolders", true),
  ...scopedRows("permissionCategoryFolders", "moveFolders", "permissionTargetOwnFolders", "permissionTargetAllFolders", true),
  ...scopedRows("permissionCategoryFolders", "deleteFolders", "permissionTargetOwnFolderBranches", "permissionTargetAllFolders", true),
  ...scopedRows("permissionCategoryFolders", "editFolderDescriptions", "permissionTargetOwnFolders", "permissionTargetAllFolders", true),
  ...scopedRows("permissionCategoryFolders", "editFolderColors", "permissionTargetOwnFolders", "permissionTargetAllFolders", true),
  ...scopedRows("permissionCategoryFolders", "editFolderIcons", "permissionTargetOwnFolders", "permissionTargetAllFolders", true),
  ...scopedRows("permissionCategoryFolders", "editFolderCovers", "permissionTargetOwnFolders", "permissionTargetAllFolders", true),
  ...scopedRows("permissionCategoryFolders", "assignFolderSmartImport", "permissionTargetOwnFolders", "permissionTargetAllFolders", true),
  { category: "permissionCategoryTags", action: "createTags", scope: "create", manager: true, editor: true, viewer: false, verified: true },
  ...scopedRows("permissionCategoryTags", "renameTags", "permissionTargetOwnTags", "permissionTargetAllTags", true),
  ...scopedRows("permissionCategoryTags", "moveTags", "permissionTargetOwnTags", "permissionTargetAllTags", true),
  ...scopedRows("permissionCategoryTags", "deleteTags", "permissionTargetOwnTags", "permissionTargetAllTags", true),
  ...scopedRows("permissionCategoryTags", "editTagColors", "permissionTargetOwnTags", "permissionTargetAllTags", true),
  ...scopedRows("permissionCategoryTags", "starTags", "permissionTargetOwnTags", "permissionTargetAllTags", true),
  { category: "permissionCategoryTags", action: "createTagGroups", scope: "create", manager: true, editor: true, viewer: false, verified: true },
  ...scopedRows("permissionCategoryTags", "renameTagGroups", "permissionTargetOwnTagGroups", "permissionTargetAllTagGroups", true),
  ...scopedRows("permissionCategoryTags", "moveTagGroups", "permissionTargetOwnTagGroups", "permissionTargetAllTagGroups", true),
  ...scopedRows("permissionCategoryTags", "editTagGroupColors", "permissionTargetOwnTagGroups", "permissionTargetAllTagGroups", true),
  ...scopedRows("permissionCategoryTags", "deleteTagGroups", "permissionTargetOwnTagGroups", "permissionTargetAllTagGroups", true),
  { category: "permissionCategorySharedPresets", action: "manageSmartFolders", scope: "all", manager: true, editor: false, viewer: false, verified: true },
  { category: "permissionCategorySharedPresets", action: "manageSmartImportRules", scope: "all", manager: true, editor: false, viewer: false, verified: true },
  { category: "permissionCategoryManagement", action: "viewMembers", scope: "all", manager: true, editor: true, viewer: true, verified: true },
  { category: "permissionCategoryManagement", action: "viewLibraryActivity", scope: "all", manager: true, editor: true, viewer: true, verified: true },
  { category: "permissionCategoryManagement", action: "addMembers", scope: "all", manager: true, editor: false, viewer: false, verified: true },
  { category: "permissionCategoryManagement", action: "removeMembers", scope: "all", manager: true, editor: false, viewer: false, verified: true },
  { category: "permissionCategoryManagement", action: "changeMemberRoles", scope: "all", manager: true, editor: false, viewer: false, verified: true },
  { category: "permissionCategoryManagement", action: "manageLibraryDetails", scope: "all", manager: true, editor: false, viewer: false, verified: true },
  { category: "permissionCategoryManagement", action: "manageLibraryIcon", scope: "all", manager: true, editor: false, viewer: false, verified: true },
  { category: "permissionCategoryManagement", action: "manageLibraryStorage", scope: "all", manager: true, editor: false, viewer: false, verified: true },
  { category: "permissionCategoryManagement", action: "deleteLibrary", scope: "all", manager: true, editor: false, viewer: false, verified: true }
] satisfies ReadonlyArray<LibraryPermissionRow>;

export const assetBreakdown = [
  { label: "images", value: 38 },
  { label: "models3d", value: 26 },
  { label: "textures", value: 21 },
  { label: "video", value: 9 },
  { label: "docs", value: 6 }
] as const;
