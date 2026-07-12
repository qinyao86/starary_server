import { useCallback, useEffect, useState } from "react";
import {
  api,
  ApiError,
  clearStoredToken,
  getStoredToken,
  storeToken,
  type ActivityItem,
  type CurrentUser,
  type LibraryMember,
  type ServerInfo,
  type StorageConnection,
  type StorageRoot,
  type TeamLibrary,
  type TeamUser
} from "../api";
import type { ApiState, DeploymentMode } from "../types";
import { canManageServerRole, isLibraryManagerRole } from "../utils/format";

export function useAdminRuntime() {
  const [deploymentMode, setDeploymentMode] = useState<DeploymentMode>("local");
  const [serviceRunning, setServiceRunning] = useState(true);
  const [apiState, setApiState] = useState<ApiState>("loading");
  const [serverInfo, setServerInfo] = useState<ServerInfo | null>(null);
  const [needsOwner, setNeedsOwner] = useState<boolean | null>(null);
  const [ownerSetupAllowed, setOwnerSetupAllowed] = useState(false);
  const [token, setToken] = useState<string | null>(() => getStoredToken());
  const [currentUser, setCurrentUser] = useState<CurrentUser | null>(null);
  const [libraries, setLibraries] = useState<TeamLibrary[]>([]);
  const [users, setUsers] = useState<TeamUser[]>([]);
  const [storageRoots, setStorageRoots] = useState<StorageRoot[]>([]);
  const [storageConnections, setStorageConnections] = useState<StorageConnection[]>([]);
  const [libraryMembers, setLibraryMembers] = useState<LibraryMember[]>([]);
  const [selectedLibraryId, setSelectedLibraryId] = useState("");
  const [assetTotal, setAssetTotal] = useState(0);
  const [activityItems, setActivityItems] = useState<ActivityItem[]>([]);
  const [libraryActivityItems, setLibraryActivityItems] = useState<ActivityItem[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const [previewMode, setPreviewMode] = useState(false);

  const loadPublicState = useCallback(async () => {
    setApiState("loading");
    try {
      const [health, info, setup] = await Promise.all([api.health(), api.serverInfo(), api.setupStatus()]);
      setServerInfo(info);
      setNeedsOwner(setup.needsOwner);
      setOwnerSetupAllowed(setup.ownerSetupAllowed);
      setServiceRunning(health.status === "ok");
      setDeploymentMode(info.deploymentMode === "local" ? "local" : "cloud");
      setApiState("connected");
    } catch (error) {
      setApiState("unavailable");
      setMessage(error instanceof Error ? error.message : String(error));
    }
  }, []);

  const loadLibraryScopedState = useCallback(async (
    nextToken: string | null,
    libraryId: string,
    canManageLibrary = true,
    libraryEnabled = true,
  ) => {
    if (!nextToken || !libraryId) {
      setStorageRoots([]);
      setLibraryMembers([]);
      setAssetTotal(0);
      setLibraryActivityItems([]);
      return;
    }

    const [roots, members, assets, activity] = await Promise.all([
      api.listStorageRoots(nextToken, libraryId),
      canManageLibrary ? api.listLibraryMembers(nextToken, libraryId) : Promise.resolve([]),
      libraryEnabled ? api.listAssets(nextToken, libraryId) : Promise.resolve({ items: [], total: 0, limit: 0, offset: 0 }),
      libraryEnabled ? api.listActivity(nextToken, libraryId) : Promise.resolve({ items: [], limit: 0, offset: 0 })
    ]);
    setStorageRoots(roots);
    setLibraryMembers(members);
    setAssetTotal(assets.total);
    setLibraryActivityItems(activity.items);
  }, []);

  const loadPrivateState = useCallback(
    async (nextToken = token) => {
      if (!nextToken) return;
      try {
        const me = await api.me(nextToken);
        const [nextLibraries, nextUsers, nextActivity, nextStorageConnections] = await Promise.all([
          api.listLibraries(nextToken),
          canManageServerRole(me.role) ? api.listUsers(nextToken) : Promise.resolve([]),
          api.listServerActivity(nextToken),
          api.listStorageConnections(nextToken)
        ]);
        setCurrentUser(me);
        setLibraries(nextLibraries);
        setUsers(nextUsers);
        setActivityItems(nextActivity.items);
        setStorageConnections(nextStorageConnections);
        const nextLibraryId = nextLibraries.some((item) => item.id === selectedLibraryId)
          ? selectedLibraryId
          : nextLibraries[0]?.id || "";
        setSelectedLibraryId(nextLibraryId);
        const nextLibrary = nextLibraries.find((item) => item.id === nextLibraryId);
        await loadLibraryScopedState(
          nextToken,
          nextLibraryId,
          isLibraryManagerRole(nextLibrary?.currentUserRole ?? me.role),
          nextLibrary?.enabled !== false,
        );
      } catch (error) {
        if (error instanceof ApiError && error.status === 401) {
          clearStoredToken();
          setToken(null);
          setCurrentUser(null);
          setActivityItems([]);
          setLibraryActivityItems([]);
        }
        setMessage(error instanceof Error ? error.message : String(error));
      }
    },
    [loadLibraryScopedState, selectedLibraryId, token]
  );

  useEffect(() => {
    void loadPublicState();
  }, [loadPublicState]);

  useEffect(() => {
    if (token) {
      void loadPrivateState(token);
    }
  }, [loadPrivateState, token]);

  useEffect(() => {
    if (!token || !currentUser) {
      return;
    }

    const reportPresence = () => {
      void api.updatePresence(token, { libraryId: selectedLibraryId || null }).catch(() => {
        // Presence is best-effort; the regular authenticated requests still surface real failures.
      });
    };

    reportPresence();
    const intervalId = window.setInterval(reportPresence, 60_000);
    return () => window.clearInterval(intervalId);
  }, [currentUser, selectedLibraryId, token]);

  useEffect(() => {
    if (!token || !canManageServerRole(currentUser?.role ?? "")) {
      return;
    }

    const refreshUsers = () => {
      void api.listUsers(token).then(setUsers).catch(() => {
        // Keep the existing list if a short polling refresh misses.
      });
    };

    const intervalId = window.setInterval(refreshUsers, 30_000);
    return () => window.clearInterval(intervalId);
  }, [currentUser?.role, token]);

  const refreshAll = async () => {
    await loadPublicState();
    await loadPrivateState(token);
  };

  const selectLibrary = (id: string) => {
    setSelectedLibraryId(id);
    if (token) {
      const library = libraries.find((item) => item.id === id);
      void loadLibraryScopedState(
        token,
        id,
        isLibraryManagerRole(library?.currentUserRole ?? currentUser?.role ?? ""),
        library?.enabled !== false,
      ).catch((error) => {
        setMessage(error instanceof Error ? error.message : String(error));
      });
    }
  };

  const onAuthenticated = (response: { accessToken: string; user: CurrentUser }) => {
    storeToken(response.accessToken);
    setToken(response.accessToken);
    setCurrentUser(response.user);
    setNeedsOwner(false);
    setMessage(null);
  };

  const logout = () => {
    clearStoredToken();
    setToken(null);
    setCurrentUser(null);
    setLibraries([]);
    setUsers([]);
    setStorageRoots([]);
    setStorageConnections([]);
    setActivityItems([]);
    setLibraryActivityItems([]);
  };

  const resetAfterInitialization = async () => {
    logout();
    setNeedsOwner(true);
    setMessage(null);
    await loadPublicState();
  };

  return {
    activityItems,
    apiState,
    assetTotal,
    currentUser,
    deploymentMode,
    libraries,
    libraryActivityItems,
    libraryMembers,
    logout,
    message,
    needsOwner,
    ownerSetupAllowed,
    onAuthenticated,
    previewMode,
    refreshAll,
    resetAfterInitialization,
    selectLibrary,
    selectedLibraryId,
    serverInfo,
    serviceRunning,
    setDeploymentMode,
    setMessage,
    setPreviewMode,
    storageRoots,
    storageConnections,
    token,
    users
  };
}
