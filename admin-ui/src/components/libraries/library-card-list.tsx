import { Copy, Database, HardDrive, Package2, Pencil, Trash2, UserRound, Users } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { Card } from "@/components/ui/card";
import emptyLibraryIllustration from "../../assets/empty-library.svg";
import type { TeamLibrary } from "../../api";
import type { TranslatorContext } from "../../types";
import { formatBytes, formatCount, formatMemberNames, storageKindLabel } from "../../utils/format";
import { EmptyState, UserAvatar } from "../common";
import { LibraryIcon } from "./library-icon";

function libraryStorageMeta(library: TeamLibrary, t: TranslatorContext["t"]) {
  const storagePath = preferredStoragePath(library);
  const kind = library.primaryStorageKind ? storageKindLabel(t, library.primaryStorageKind) : t("storage");
  const extraCount = Math.max((library.storageRootCount ?? 0) - 1, 0);
  return {
    extraCount,
    kind,
    path: storagePath || t("noStorageLocation"),
    rawPath: storagePath
  };
}

function preferredStoragePath(library: TeamLibrary) {
  const platform = currentPlatformLabel();
  if (platform.includes("mac")) {
    return library.primaryStorageMacosPath
      || smbUrlFromUnc(library.primaryStorageWindowsPath)
      || library.primaryStorageUri
      || "";
  }
  if (platform.includes("win")) {
    return library.primaryStorageWindowsPath
      || uncFromSmbUrl(library.primaryStorageUri)
      || uncFromSmbUrl(library.primaryStorageMacosPath)
      || library.primaryStorageUri
      || "";
  }
  return library.primaryStorageUri || library.primaryStorageMacosPath || library.primaryStorageWindowsPath || "";
}

function currentPlatformLabel() {
  if (typeof navigator === "undefined") return "";
  return `${navigator.platform} ${navigator.userAgent}`.toLowerCase();
}

function uncFromSmbUrl(value?: string | null) {
  if (!value?.startsWith("smb://")) return "";
  const parts = value
    .slice("smb://".length)
    .split("/")
    .filter(Boolean);
  if (parts.length < 2) return "";
  return `\\\\${parts.join("\\")}`;
}

function smbUrlFromUnc(value?: string | null) {
  if (!value?.startsWith("\\\\")) return "";
  const parts = value
    .replace(/\//g, "\\")
    .split("\\")
    .filter(Boolean);
  if (parts.length < 2) return "";
  return `smb://${parts.join("/")}`;
}

function LibraryMetric({ icon: Icon, label, value }: { icon: LucideIcon; label: string; value: string }) {
  return (
    <div className="library-card-metric">
      <div className="library-card-metric-heading">
        <Icon aria-hidden="true" size={15} />
        <span className="library-card-metric-label">{label}</span>
      </div>
      <strong>{value}</strong>
    </div>
  );
}

async function copyTextToClipboard(value: string) {
  if (!value.trim()) return;

  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(value);
    return;
  }

  const textarea = document.createElement("textarea");
  textarea.value = value;
  textarea.setAttribute("readonly", "true");
  textarea.style.position = "fixed";
  textarea.style.left = "-9999px";
  document.body.appendChild(textarea);
  textarea.select();
  document.execCommand("copy");
  document.body.removeChild(textarea);
}

export function LibraryCardList({
  libraries,
  canDeleteLibrary,
  canManageLibrary,
  token,
  iconUpdatingLibraryId,
  t,
  onOpen,
  onDelete,
  onEdit,
  onChangeIcon,
  onToggleEnabled
}: TranslatorContext & {
  libraries: TeamLibrary[];
  canDeleteLibrary: boolean;
  canManageLibrary: (library: TeamLibrary) => boolean;
  token: string | null;
  iconUpdatingLibraryId: string | null;
  onDelete: (libraryId: string) => void;
  onOpen: (libraryId: string) => void;
  onEdit: (libraryId: string) => void;
  onChangeIcon: (libraryId: string) => void;
  onToggleEnabled: (library: TeamLibrary, enabled: boolean) => void;
}) {
  if (libraries.length === 0) {
    return <EmptyState illustration={emptyLibraryIllustration} label={t("noLibraries")} />;
  }

  return (
    <div className="library-card-grid">
      {libraries.map((library) => {
        const isEnabled = library.enabled !== false;
        const canManage = canManageLibrary(library);
        const storageMeta = libraryStorageMeta(library, t);
        return (
          <Card
            className={`library-card${isEnabled ? "" : " is-disabled"}${canManage ? " has-switch" : ""}`}
            key={library.id}
            role="button"
            tabIndex={0}
            onClick={() => onOpen(library.id)}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                onOpen(library.id);
              }
            }}
          >
            <div className="library-card-top">
              <div className="library-card-identity">
                <LibraryIcon
                  busy={iconUpdatingLibraryId === library.id}
                  editable={canManage}
                  library={library}
                  t={t}
                  token={token}
                  onChange={() => onChangeIcon(library.id)}
                />
                <div className="library-card-heading">
                  <div className="library-card-title-row">
                    <div className="library-card-title">{library.displayName}</div>
                  </div>
                  <div className="library-card-title-meta">
                    <span className={`library-card-status${isEnabled ? " is-on" : " is-off"}`}>
                      {isEnabled ? t("libraryStarted") : t("libraryNotStarted")}
                    </span>
                    <span className={`badge library-card-access-badge is-${library.accessMode === "public" ? "public" : "invite"}`}>
                      {library.accessMode === "public" ? t("libraryAccessPublic") : t("libraryAccessInvite")}
                    </span>
                  </div>
                </div>
              </div>
              <div className="library-card-meta-group">
                <div className="library-card-path-row">
                  <HardDrive aria-hidden="true" size={14} />
                  <span className="library-card-storage-kind">
                    {storageMeta.kind}
                    {storageMeta.extraCount > 0 ? ` +${storageMeta.extraCount}` : ""}
                  </span>
                  <strong
                    className="library-card-storage-path"
                    title={storageMeta.path}
                    onClick={(event) => event.stopPropagation()}
                    onKeyDown={(event) => event.stopPropagation()}
                  >
                    {storageMeta.path}
                  </strong>
                  <button
                    aria-label={t("copyMode")}
                    className="library-card-storage-tool"
                    disabled={!storageMeta.rawPath}
                    title={t("copyMode")}
                    type="button"
                    onClick={(event) => {
                      event.stopPropagation();
                      void copyTextToClipboard(storageMeta.rawPath);
                    }}
                    onKeyDown={(event) => event.stopPropagation()}
                  >
                    <Copy size={14} />
                  </button>
                </div>
                <div className="library-card-manager-row">
                  <UserRound aria-hidden="true" size={14} />
                  <span>{t("libraryManager")}</span>
                  <div className="library-card-manager-users" title={formatMemberNames(library.libraryManagerNames, "-")}>
                    {(library.libraryManagerNames ?? []).length === 0 ? (
                      <strong>-</strong>
                    ) : (
                      <>
                        {(library.libraryManagerNames ?? []).slice(0, 3).map((name, index) => (
                          <span className="library-card-manager-user" key={`${name}-${index}`}>
                            <UserAvatar avatarKey={library.libraryManagerAvatarKeys?.[index]} label={name} size="sm" />
                            <strong>{name}</strong>
                          </span>
                        ))}
                        {(library.libraryManagerNames ?? []).length > 3 && (
                          <strong>+{(library.libraryManagerNames ?? []).length - 3}</strong>
                        )}
                      </>
                    )}
                  </div>
                </div>
              </div>
              {canManage && (
                <button
                  aria-checked={isEnabled}
                  aria-label={isEnabled ? t("deactivate") : t("activate")}
                  className={`library-card-switch${isEnabled ? " is-on" : ""}`}
                  role="switch"
                  type="button"
                  onClick={(event) => {
                    event.stopPropagation();
                    onToggleEnabled(library, !isEnabled);
                  }}
                  onKeyDown={(event) => event.stopPropagation()}
                >
                  <span />
                </button>
              )}
            </div>
            <div className="library-card-metrics">
              <LibraryMetric icon={Package2} label={t("assets")} value={formatCount(library.assetCount)} />
              <LibraryMetric icon={Database} label={t("totalSize")} value={formatBytes(library.totalSizeBytes)} />
              <LibraryMetric icon={Users} label={t("membersLabel")} value={formatCount(library.memberNames?.length)} />
            </div>
            {(canManage || canDeleteLibrary) && (
              <div className="library-card-footer">
                <div className="library-card-action-buttons">
                  {canManage && (
                    <button
                      aria-label={t("editLibrary")}
                      className="library-card-action-button"
                      title={t("editLibrary")}
                      type="button"
                      onClick={(event) => {
                        event.stopPropagation();
                        onEdit(library.id);
                      }}
                      onKeyDown={(event) => event.stopPropagation()}
                    >
                      <Pencil size={15} />
                      <span>{t("editLibrary")}</span>
                    </button>
                  )}
                  {canDeleteLibrary && (
                    <button
                      aria-label={t("deleteLibrary")}
                      className="library-card-action-button is-danger"
                      title={t("deleteLibrary")}
                      type="button"
                      onClick={(event) => {
                        event.stopPropagation();
                        onDelete(library.id);
                      }}
                      onKeyDown={(event) => event.stopPropagation()}
                    >
                      <Trash2 size={15} />
                      <span>{t("deleteLibrary")}</span>
                    </button>
                  )}
                </div>
              </div>
            )}
          </Card>
        );
      })}
    </div>
  );
}
