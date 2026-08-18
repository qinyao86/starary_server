const APP_VERSION = "1.0.0";
const UPDATE_MANIFEST_URL = "https://starary.com/updates/server.json";
const DEFAULT_RELEASE_URL = "https://starary.com/";

function normalizeVersion(value) {
  return String(value ?? "").trim().replace(/^v/i, "").replace(/\+.*$/u, "");
}

function parseVersion(value) {
  const normalized = normalizeVersion(value);
  const match = normalized.match(
    /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z-]*))*))?$/u,
  );
  if (!match) {
    return null;
  }

  return {
    preReleaseIdentifiers: match[4]?.split(".") ?? [],
    segments: [Number(match[1]), Number(match[2]), Number(match[3])],
  };
}

function compareVersions(leftVersion, rightVersion) {
  const left = parseVersion(leftVersion);
  const right = parseVersion(rightVersion);
  if (!left || !right) {
    return null;
  }

  for (let index = 0; index < 3; index += 1) {
    if (left.segments[index] !== right.segments[index]) {
      return left.segments[index] - right.segments[index];
    }
  }

  if (left.preReleaseIdentifiers.length > 0 && right.preReleaseIdentifiers.length === 0) {
    return -1;
  }
  if (left.preReleaseIdentifiers.length === 0 && right.preReleaseIdentifiers.length > 0) {
    return 1;
  }

  const identifierCount = Math.max(
    left.preReleaseIdentifiers.length,
    right.preReleaseIdentifiers.length,
  );
  for (let index = 0; index < identifierCount; index += 1) {
    const leftIdentifier = left.preReleaseIdentifiers[index];
    const rightIdentifier = right.preReleaseIdentifiers[index];
    if (leftIdentifier === undefined) {
      return -1;
    }
    if (rightIdentifier === undefined) {
      return 1;
    }
    if (leftIdentifier === rightIdentifier) {
      continue;
    }

    const leftIsNumeric = /^\d+$/u.test(leftIdentifier);
    const rightIsNumeric = /^\d+$/u.test(rightIdentifier);
    if (leftIsNumeric && rightIsNumeric) {
      return Number(leftIdentifier) - Number(rightIdentifier);
    }
    if (leftIsNumeric) {
      return -1;
    }
    if (rightIsNumeric) {
      return 1;
    }
    return leftIdentifier.localeCompare(rightIdentifier);
  }

  return 0;
}

function resolveLocalizedNote(notes, language) {
  if (!notes) {
    return undefined;
  }
  if (typeof notes === "string") {
    return notes.trim() || undefined;
  }

  const localeKey = language === "zh" ? "zh-CN" : "en";
  return (
    notes[localeKey]?.trim() ||
    notes["zh-CN"]?.trim() ||
    notes.en?.trim() ||
    Object.values(notes).find((value) => typeof value === "string" && value.trim())?.trim()
  );
}

function resolveChannel(manifest, channel = "stable") {
  return manifest.channels?.[channel] ?? (channel === "stable" ? manifest.stable : undefined) ?? manifest.stable;
}

function resolveReleaseUrl(channel, manifest) {
  const candidate = channel?.releaseUrl ?? manifest.releaseUrl ?? DEFAULT_RELEASE_URL;
  try {
    const url = new URL(candidate);
    return url.protocol === "http:" || url.protocol === "https:" ? url.toString() : DEFAULT_RELEASE_URL;
  } catch {
    return DEFAULT_RELEASE_URL;
  }
}

export async function checkForServerUpdate({
  currentVersion = APP_VERSION,
  language = "zh",
  timeoutMs = 8000,
  signal,
} = {}) {
  const controller = new AbortController();
  const abortFromCaller = () => controller.abort();
  if (signal?.aborted) {
    controller.abort();
  } else {
    signal?.addEventListener("abort", abortFromCaller, { once: true });
  }
  const timeoutId = window.setTimeout(() => controller.abort(), timeoutMs);

  try {
    const response = await fetch(UPDATE_MANIFEST_URL, {
      cache: "no-store",
      credentials: "omit",
      headers: {
        Accept: "application/json",
      },
      signal: controller.signal,
    });
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }

    const manifest = await response.json();
    const selectedChannel = resolveChannel(manifest);
    const latestVersion = normalizeVersion(selectedChannel?.version ?? manifest.version ?? "");
    if (!latestVersion) {
      throw new Error("Update manifest missing version.");
    }

    const comparison = compareVersions(currentVersion, latestVersion);
    if (comparison === null) {
      throw new Error("Invalid version format.");
    }

    return {
      currentVersion: normalizeVersion(currentVersion),
      latestVersion,
      isUpdateAvailable: comparison < 0,
      publishedAt: selectedChannel?.publishedAt ?? manifest.publishedAt,
      releaseUrl: resolveReleaseUrl(selectedChannel, manifest),
      note: resolveLocalizedNote(selectedChannel?.notes ?? manifest.notes, language),
    };
  } finally {
    window.clearTimeout(timeoutId);
    signal?.removeEventListener("abort", abortFromCaller);
  }
}

export { APP_VERSION, UPDATE_MANIFEST_URL, DEFAULT_RELEASE_URL };
