import type { StorageRoot } from "../api";

export function storageRootPayloadFromRecord(root: StorageRoot, enabled = root.enabled) {
  return {
    name: root.name,
    kind: root.kind,
    canonicalUri: root.canonicalUri,
    windowsUncPath: root.windowsUncPath ?? undefined,
    windowsMappedDriveAliases: root.windowsMappedDriveAliases,
    macosSmbUrl: root.macosSmbUrl ?? undefined,
    macosMountAliases: root.macosMountAliases,
    enabled
  };
}
