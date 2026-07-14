export type ServerInfo = {
  product: string;
  version: string;
  apiVersion: string;
  deploymentMode: string;
  serverUrl: string;
  localUrl: string;
  lanUrl?: string | null;
  localAdminUrl: string;
  lanAdminUrl?: string | null;
  bindAddress: string;
  storageDir: string;
  adminAvailable: boolean;
  databaseStatus: string;
  storageStatus: string;
  storageWritable: boolean;
};

export type SetupStatus = {
  needsOwner: boolean;
  ownerSetupAllowed: boolean;
};

export type CurrentUser = {
  id: string;
  email: string;
  displayName: string;
  role: string;
};

export type LoginResponse = {
  accessToken: string;
  tokenType: string;
  user: CurrentUser;
};

export type TeamLibrary = {
  id: string;
  displayName: string;
  currentUserRole?: string;
  enabled: boolean;
  storageLockedAt?: string | null;
  libraryManagerNames?: string[];
  memberNames?: string[];
  assetCount?: number;
  folderCount?: number;
  tagCount?: number;
  totalSizeBytes?: number;
  storageRootCount?: number;
  enabledStorageRootCount?: number;
  primaryStorageKind?: string | null;
  primaryStorageConnectionId?: string | null;
  primaryStorageConnectionName?: string | null;
  primaryStorageNamespace?: string | null;
  primaryStorageUri?: string | null;
  primaryStorageWindowsPath?: string | null;
  primaryStorageMacosPath?: string | null;
  createdByUserId?: string;
  createdAt?: string;
  updatedAt?: string;
};

export type RuntimeSettings = {
  currentPort: number;
  configuredPort: number;
  restartRequired: boolean;
  serviceControlAvailable: boolean;
};

export type BackupSettings = {
  automaticEnabled: boolean;
  automaticTime: string;
  retentionCount: number;
};

export type BackupStatus = {
  available: boolean;
  backupDir: string;
  settings: BackupSettings;
};

export type BackupRecord = {
  id: string;
  kind: "automatic" | "manual" | "pre_restore" | "pre_initialize";
  sizeBytes: number;
  createdAt: string;
};

export type BackupOverview = {
  status: BackupStatus;
  backups: BackupRecord[];
};

export type StorageConnection = {
  id: string;
  name: string;
  kind: string;
  canonicalUri: string;
  windowsUncPath?: string | null;
  windowsMappedDriveAliases: string[];
  macosSmbUrl?: string | null;
  macosMountAliases: string[];
  enabled: boolean;
  isDefault: boolean;
  libraryCount: number;
  libraryNames: string[];
  assetCount: number;
  totalSizeBytes: number;
  createdByUserId: string;
  createdAt: string;
  updatedAt: string;
};

export type StorageConnectionInput = {
  name?: string;
  kind: string;
  canonicalUri: string;
  windowsUncPath?: string;
  windowsMappedDriveAliases?: string[];
  macosSmbUrl?: string;
  macosMountAliases?: string[];
};

export type StorageMigrationResult = {
  connection: StorageConnection;
  migratedLibraryCount: number;
  migratedAssetCount: number;
  estimatedSizeBytes: number;
  previousLocation: string;
  currentLocation: string;
};

export type TeamUser = {
  id: string;
  email: string;
  displayName: string;
  globalRole: string;
  isActive: boolean;
  lastLoginAt?: string | null;
  lastSeenAt?: string | null;
  lastSeenLibraryId?: string | null;
  lastSeenLibraryName?: string | null;
  libraryMemberships?: UserLibraryMembership[];
  createdAt: string;
  updatedAt: string;
};

export type UserLibraryMembership = {
  libraryId: string;
  libraryName: string;
  role: string;
};

export type StorageRoot = {
  id: string;
  libraryId: string;
  storageConnectionId: string;
  storageConnectionName: string;
  namespace: string;
  name: string;
  kind: string;
  canonicalUri: string;
  windowsUncPath?: string | null;
  windowsMappedDriveAliases: string[];
  macosSmbUrl?: string | null;
  macosMountAliases: string[];
  enabled: boolean;
  createdByUserId: string;
  createdAt: string;
  updatedAt: string;
};

export type DefaultStorageRootInput = {
  name?: string;
  kind: string;
  canonicalUri: string;
  windowsUncPath?: string;
  windowsMappedDriveAliases?: string[];
  macosSmbUrl?: string;
  macosMountAliases?: string[];
};

export type LibraryMember = {
  libraryId: string;
  userId: string;
  email: string;
  displayName: string;
  role: string;
  createdAt: string;
  updatedAt: string;
};

export type AssetListResponse = {
  items: unknown[];
  total: number;
  limit: number;
  offset: number;
};

export type ActivityItem = {
  id: string;
  libraryId?: string | null;
  actorUserId?: string | null;
  actorDisplayName?: string | null;
  actorEmail?: string | null;
  action: string;
  targetType: string;
  targetId?: string | null;
  targetName?: string | null;
  details: Record<string, unknown>;
  createdAt: string;
};

export type ActivityListResponse = {
  items: ActivityItem[];
  limit: number;
  offset: number;
};
