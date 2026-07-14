import { request, requestBlob } from "./request";
import type {
  ActivityListResponse,
  AssetListResponse,
  BackupOverview,
  BackupRecord,
  BackupSettings,
  BackupStatus,
  CurrentUser,
  DefaultStorageRootInput,
  LibraryMember,
  LoginResponse,
  RuntimeSettings,
  ServerInfo,
  SetupStatus,
  StorageConnection,
  StorageConnectionInput,
  StorageMigrationResult,
  StorageRoot,
  TeamLibrary,
  TeamUser
} from "./types";

export const api = {
  health: () => request<{ status: string }>("/health"),
  serverInfo: () => request<ServerInfo>("/api/v1/server/info"),
  runtimeSettings: (token: string) => request<RuntimeSettings>("/api/v1/server/runtime", { token }),
  updateRuntimeSettings: (token: string, payload: { port: number }) =>
    request<RuntimeSettings>("/api/v1/server/runtime", {
      method: "PATCH",
      token,
      body: JSON.stringify(payload)
    }),
  shutdownServer: (token: string) =>
    request<void>("/api/v1/server/shutdown", { method: "POST", token }),
  setupStatus: () => request<SetupStatus>("/api/v1/setup/status"),
  backupOverview: (token: string) => request<BackupOverview>("/api/v1/backups", { token }),
  updateBackupSettings: (token: string, payload: BackupSettings) =>
    request<BackupStatus>("/api/v1/backups/settings", {
      method: "PATCH",
      token,
      body: JSON.stringify(payload)
    }),
  createBackup: (token: string) =>
    request<BackupRecord>("/api/v1/backups", { method: "POST", token }),
  downloadBackup: (token: string, backupId: string) =>
    requestBlob(`/api/v1/backups/${encodeURIComponent(backupId)}`, token),
  deleteBackup: (token: string, backupId: string) =>
    request<void>(`/api/v1/backups/${encodeURIComponent(backupId)}`, { method: "DELETE", token }),
  restoreBackup: (token: string, backupId: string) =>
    request<void>("/api/v1/backups/restore", {
      method: "POST",
      token,
      body: JSON.stringify({ backupId })
    }),
  initializeServer: (token: string) =>
    request<void>("/api/v1/server/initialize", { method: "POST", token }),
  createOwner: (payload: { email: string; password: string; displayName?: string }) =>
    request<LoginResponse>("/api/v1/setup/owner", {
      method: "POST",
      body: JSON.stringify(payload)
    }),
  login: (payload: { email: string; password: string }) =>
    request<LoginResponse>("/api/v1/auth/login", {
      method: "POST",
      body: JSON.stringify(payload)
    }),
  createBrowserHandoff: (token: string) =>
    request<{ code: string }>("/api/v1/auth/browser-handoff", { method: "POST", token }),
  redeemBrowserHandoff: (code: string) =>
    request<LoginResponse>("/api/v1/auth/browser-handoff/redeem", {
      method: "POST",
      body: JSON.stringify({ code })
    }),
  me: (token: string) => request<CurrentUser>("/api/v1/me", { token }),
  updatePresence: (token: string, payload: { libraryId?: string | null } = {}) =>
    request<void>("/api/v1/me/presence", {
      method: "PATCH",
      token,
      body: JSON.stringify(payload)
    }),
  listUsers: (token: string) => request<TeamUser[]>("/api/v1/users", { token }),
  createUser: (token: string, payload: { email: string; password: string; displayName?: string; role?: string; libraryMemberships?: Array<{ libraryId: string; role: string }> }) =>
    request<TeamUser>("/api/v1/users", {
      method: "POST",
      token,
      body: JSON.stringify(payload)
    }),
  updateUser: (token: string, userId: string, payload: { displayName?: string; role?: string; isActive?: boolean; password?: string; libraryMemberships?: Array<{ libraryId: string; role: string }> }) =>
    request<TeamUser>(`/api/v1/users/${userId}`, {
      method: "PATCH",
      token,
      body: JSON.stringify(payload)
    }),
  listLibraries: (token: string) => request<TeamLibrary[]>("/api/v1/libraries", { token }),
  createLibrary: (token: string, payload: { displayName: string; defaultStorageRoot?: DefaultStorageRootInput; storageBinding?: { connectionId: string; namespace?: string } }) =>
    request<TeamLibrary>("/api/v1/libraries", {
      method: "POST",
      token,
      body: JSON.stringify(payload)
    }),
  updateLibrary: (token: string, libraryId: string, payload: { displayName: string; iconUrl?: string | null }) =>
    request<TeamLibrary>(`/api/v1/libraries/${libraryId}`, {
      method: "PATCH",
      token,
      body: JSON.stringify(payload)
    }),
  updateLibraryEnabled: (token: string, libraryId: string, payload: { enabled: boolean }) =>
    request<TeamLibrary>(`/api/v1/libraries/${libraryId}/enabled`, {
      method: "PATCH",
      token,
      body: JSON.stringify(payload)
    }),
  assignLibraryStorage: (token: string, libraryId: string, payload: { connectionId: string; namespace?: string }) =>
    request<void>(`/api/v1/libraries/${libraryId}/storage-binding`, {
      method: "POST",
      token,
      body: JSON.stringify(payload)
    }),
  deleteLibrary: (token: string, libraryId: string, payload: { deleteFiles: boolean }) =>
    request<void>(`/api/v1/libraries/${libraryId}`, {
      method: "DELETE",
      token,
      body: JSON.stringify(payload)
    }),
  listLibraryMembers: (token: string, libraryId: string) =>
    request<LibraryMember[]>(`/api/v1/libraries/${libraryId}/members`, { token }),
  upsertLibraryMember: (token: string, libraryId: string, userId: string, payload: { role: string }) =>
    request<LibraryMember>(`/api/v1/libraries/${libraryId}/members/${userId}`, {
      method: "POST",
      token,
      body: JSON.stringify(payload)
    }),
  removeLibraryMember: (token: string, libraryId: string, userId: string) =>
    request<void>(`/api/v1/libraries/${libraryId}/members/${userId}`, {
      method: "DELETE",
      token
    }),
  listStorageConnections: (token: string) => request<StorageConnection[]>("/api/v1/storage-connections", { token }),
  createStorageConnection: (token: string, payload: StorageConnectionInput) =>
    request<StorageConnection>("/api/v1/storage-connections", {
      method: "POST",
      token,
      body: JSON.stringify(payload)
    }),
  updateStorageConnection: (token: string, connectionId: string, payload: StorageConnectionInput & { enabled: boolean }) =>
    request<StorageConnection>(`/api/v1/storage-connections/${connectionId}`, {
      method: "PATCH",
      token,
      body: JSON.stringify(payload)
    }),
  migrateStorageConnection: (token: string, connectionId: string, payload: StorageConnectionInput) =>
    request<StorageMigrationResult>(`/api/v1/storage-connections/${connectionId}/migrate`, {
      method: "POST",
      token,
      body: JSON.stringify(payload)
    }),
  setDefaultStorageConnection: (token: string, connectionId: string) =>
    request<StorageConnection>(`/api/v1/storage-connections/${connectionId}/default`, {
      method: "PATCH",
      token
    }),
  deleteStorageConnection: (token: string, connectionId: string) =>
    request<void>(`/api/v1/storage-connections/${connectionId}`, {
      method: "DELETE",
      token
    }),
  listStorageRoots: (token: string, libraryId: string) =>
    request<StorageRoot[]>(`/api/v1/storage-roots?libraryId=${encodeURIComponent(libraryId)}`, { token }),
  listAssets: (token: string, libraryId: string) =>
    request<AssetListResponse>(`/api/v1/libraries/${libraryId}/assets?limit=1&offset=0`, { token }),
  listServerActivity: (token: string) =>
    request<ActivityListResponse>("/api/v1/activity?limit=20&offset=0", { token }),
  listActivity: (token: string, libraryId: string) =>
    request<ActivityListResponse>(`/api/v1/libraries/${libraryId}/activity?limit=10&offset=0`, { token })
};
