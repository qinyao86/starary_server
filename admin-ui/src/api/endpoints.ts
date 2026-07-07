import { request } from "./request";
import type {
  ActivityListResponse,
  AssetListResponse,
  CurrentUser,
  LibraryMember,
  LoginResponse,
  ServerInfo,
  SetupStatus,
  StorageRoot,
  TeamLibrary,
  TeamUser
} from "./types";

export const api = {
  health: () => request<{ status: string }>("/health"),
  serverInfo: () => request<ServerInfo>("/api/v1/server/info"),
  setupStatus: () => request<SetupStatus>("/api/v1/setup/status"),
  createOwner: (payload: { email: string; password: string; displayName?: string }) =>
    request<LoginResponse & { defaultLibraryId: string }>("/api/v1/setup/owner", {
      method: "POST",
      body: JSON.stringify(payload)
    }),
  login: (payload: { email: string; password: string }) =>
    request<LoginResponse>("/api/v1/auth/login", {
      method: "POST",
      body: JSON.stringify(payload)
    }),
  me: (token: string) => request<CurrentUser>("/api/v1/me", { token }),
  updatePresence: (token: string, payload: { libraryId?: string | null } = {}) =>
    request<void>("/api/v1/me/presence", {
      method: "PATCH",
      token,
      body: JSON.stringify(payload)
    }),
  listUsers: (token: string) => request<TeamUser[]>("/api/v1/users", { token }),
  createUser: (token: string, payload: { email: string; password: string; displayName?: string; role?: string }) =>
    request<TeamUser>("/api/v1/users", {
      method: "POST",
      token,
      body: JSON.stringify(payload)
    }),
  updateUser: (token: string, userId: string, payload: { displayName?: string; role?: string; isActive?: boolean; password?: string }) =>
    request<TeamUser>(`/api/v1/users/${userId}`, {
      method: "PATCH",
      token,
      body: JSON.stringify(payload)
    }),
  listLibraries: (token: string) => request<TeamLibrary[]>("/api/v1/libraries", { token }),
  createLibrary: (token: string, payload: { displayName: string; description?: string }) =>
    request<TeamLibrary>("/api/v1/libraries", {
      method: "POST",
      token,
      body: JSON.stringify(payload)
    }),
  updateLibrary: (token: string, libraryId: string, payload: { displayName: string; description?: string }) =>
    request<TeamLibrary>(`/api/v1/libraries/${libraryId}`, {
      method: "PATCH",
      token,
      body: JSON.stringify(payload)
    }),
  deleteLibrary: (token: string, libraryId: string) =>
    request<void>(`/api/v1/libraries/${libraryId}`, {
      method: "DELETE",
      token
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
  listStorageRoots: (token: string, libraryId: string) =>
    request<StorageRoot[]>(`/api/v1/storage-roots?libraryId=${encodeURIComponent(libraryId)}`, { token }),
  createStorageRoot: (
    token: string,
    payload: {
      libraryId: string;
      name: string;
      kind: string;
      canonicalUri: string;
      windowsUncPath?: string;
      windowsMappedDriveAliases?: string[];
      macosSmbUrl?: string;
      macosMountAliases?: string[];
    }
  ) =>
    request<StorageRoot>("/api/v1/storage-roots", {
      method: "POST",
      token,
      body: JSON.stringify(payload)
    }),
  updateStorageRoot: (
    token: string,
    rootId: string,
    payload: {
      name: string;
      kind: string;
      canonicalUri: string;
      windowsUncPath?: string;
      windowsMappedDriveAliases?: string[];
      macosSmbUrl?: string;
      macosMountAliases?: string[];
      enabled: boolean;
    }
  ) =>
    request<StorageRoot>(`/api/v1/storage-roots/${rootId}`, {
      method: "PATCH",
      token,
      body: JSON.stringify(payload)
    }),
  deleteStorageRoot: (token: string, rootId: string) =>
    request<void>(`/api/v1/storage-roots/${rootId}`, {
      method: "DELETE",
      token
    }),
  listAssets: (token: string, libraryId: string) =>
    request<AssetListResponse>(`/api/v1/libraries/${libraryId}/assets?limit=1&offset=0`, { token }),
  listServerActivity: (token: string) =>
    request<ActivityListResponse>("/api/v1/activity?limit=20&offset=0", { token }),
  listActivity: (token: string, libraryId: string) =>
    request<ActivityListResponse>(`/api/v1/libraries/${libraryId}/activity?limit=10&offset=0`, { token })
};
