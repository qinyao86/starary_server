export type ServerInfo = {
  product: string;
  version: string;
  apiVersion: string;
  deploymentMode: string;
  serverUrl: string;
  storageDir: string;
  adminAvailable: boolean;
  databaseStatus: string;
  storageStatus: string;
  storageWritable: boolean;
};

export type SetupStatus = {
  needsOwner: boolean;
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
  name: string;
  description?: string | null;
  currentUserRole?: string;
  creatorName?: string;
  memberNames?: string[];
  assetCount?: number;
  folderCount?: number;
  tagCount?: number;
  totalSizeBytes?: number;
  createdByUserId?: string;
  createdAt?: string;
  updatedAt?: string;
};

export type TeamUser = {
  id: string;
  email: string;
  displayName: string;
  globalRole: string;
  isActive: boolean;
  createdAt: string;
  updatedAt: string;
};

export type StorageRoot = {
  id: string;
  libraryId: string;
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

const tokenKey = "madlibrary_server_admin_token";

export function getStoredToken() {
  return localStorage.getItem(tokenKey);
}

export function storeToken(token: string) {
  localStorage.setItem(tokenKey, token);
}

export function clearStoredToken() {
  localStorage.removeItem(tokenKey);
}

async function request<T>(path: string, options: RequestInit & { token?: string | null } = {}): Promise<T> {
  const headers = new Headers(options.headers);
  headers.set("Accept", "application/json");
  if (options.body && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }
  if (options.token) {
    headers.set("Authorization", `Bearer ${options.token}`);
  }

  const response = await fetch(path, { ...options, headers });
  if (!response.ok) {
    let message = `${response.status} ${response.statusText}`;
    try {
      const body = (await response.json()) as { error?: string };
      if (body.error) message = body.error;
    } catch {
      // Keep the HTTP status fallback.
    }
    throw new Error(message);
  }

  if (response.status === 204) {
    return undefined as T;
  }
  return (await response.json()) as T;
}

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
  listUsers: (token: string) => request<TeamUser[]>("/api/v1/users", { token }),
  createUser: (
    token: string,
    payload: { email: string; password: string; displayName?: string; role?: string }
  ) =>
    request<TeamUser>("/api/v1/users", {
      method: "POST",
      token,
      body: JSON.stringify(payload)
    }),
  updateUser: (
    token: string,
    userId: string,
    payload: { displayName?: string; role?: string; isActive?: boolean; password?: string }
  ) =>
    request<TeamUser>(`/api/v1/users/${userId}`, {
      method: "PATCH",
      token,
      body: JSON.stringify(payload)
    }),
  listLibraries: (token: string) => request<TeamLibrary[]>("/api/v1/libraries", { token }),
  createLibrary: (token: string, payload: { name: string; description?: string }) =>
    request<TeamLibrary>("/api/v1/libraries", {
      method: "POST",
      token,
      body: JSON.stringify(payload)
    }),
  updateLibrary: (token: string, libraryId: string, payload: { name: string; description?: string }) =>
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
  listActivity: (token: string, libraryId: string) =>
    request<ActivityListResponse>(`/api/v1/libraries/${libraryId}/activity?limit=10&offset=0`, { token })
};
