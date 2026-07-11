import { Copy, ExternalLink, HardDrive, Images, Pencil, Trash2, UserRound, Users } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import type { TeamLibrary } from "../../api";
import type { TranslatorContext } from "../../types";
import { formatBytes, formatCount, storageKindLabel } from "../../utils/format";

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
      <span className="library-card-metric-label"><Icon aria-hidden="true" size={13} />{label}</span>
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
  canManageLibrary,
  t,
  onDelete,
  onEdit,
  onOpen,
  onToggleEnabled
}: TranslatorContext & {
  libraries: TeamLibrary[];
  canManageLibrary: (library: TeamLibrary) => boolean;
  onDelete: (library: TeamLibrary) => void;
  onEdit: (library: TeamLibrary) => void;
  onOpen: (libraryId: string) => void;
  onToggleEnabled: (library: TeamLibrary, enabled: boolean) => void;
}) {
  if (libraries.length === 0) {
    return <div className="placeholder-box">{t("noLibraries")}</div>;
  }

  return (
    <div className="library-card-grid">
      {libraries.map((library) => {
        const isEnabled = library.enabled !== false;
        const canManage = canManageLibrary(library);
        const storageMeta = libraryStorageMeta(library, t);
        return (
          <Card className={`library-card${isEnabled ? "" : " is-disabled"}`} key={library.id}>
            <div className="library-card-top">
              <div className="library-card-heading">
                <div className="library-card-title-row">
                  <div className="library-card-title">{library.displayName}</div>
                  <span className={`library-card-status${isEnabled ? " is-on" : " is-off"}`}>
                    {isEnabled ? t("libraryStarted") : t("libraryNotStarted")}
                  </span>
                </div>
                <div className="library-card-description">{library.description || t("noDescription")}</div>
              </div>
              {canManage && <button
                aria-checked={isEnabled}
                aria-label={isEnabled ? t("deactivate") : t("activate")}
                className={`library-card-switch${isEnabled ? " is-on" : ""}`}
                role="switch"
                type="button"
                onClick={() => onToggleEnabled(library, !isEnabled)}
              >
                <span />
              </button>}
            </div>

            <div className="library-card-storage">
              <div className="library-card-storage-header">
                <span>{t("storageLocation")}</span>
                <em>
                  {storageMeta.kind}
                  {storageMeta.extraCount > 0 ? ` +${storageMeta.extraCount}` : ""}
                </em>
              </div>
              <div className="library-card-storage-path-row">
                <strong className="library-card-storage-path" title={storageMeta.path}>
                  {storageMeta.path}
                </strong>
                <button
                  aria-label={t("copyMode")}
                  className="library-card-storage-tool"
                  disabled={!storageMeta.rawPath}
                  title={t("copyMode")}
                  type="button"
                  onClick={() => void copyTextToClipboard(storageMeta.rawPath)}
                >
                  <Copy size={14} />
                </button>
              </div>
            </div>

            <div className="library-card-metrics">
              <LibraryMetric icon={HardDrive} label={t("totalSize")} value={formatBytes(library.totalSizeBytes)} />
              <LibraryMetric icon={Users} label={t("membersLabel")} value={formatCount(library.memberNames?.length)} />
              <LibraryMetric icon={Images} label={t("assets")} value={formatCount(library.assetCount)} />
              <LibraryMetric icon={UserRound} label={t("creator")} value={library.creatorName || "-"} />
            </div>

            <div className="library-card-actions">
              <Button className="library-card-action is-primary" size="sm" type="button" onClick={() => onOpen(library.id)}>
                <ExternalLink size={15} />
                <span>{t("openLibrary")}</span>
              </Button>
              {canManage && <Button className="library-card-action" size="sm" type="button" variant="outline" onClick={() => onEdit(library)}>
                <Pencil size={15} />
                <span>{t("editLibrary")}</span>
              </Button>}
              {canManage && <Button className="library-card-action is-danger" size="sm" type="button" variant="outline" onClick={() => onDelete(library)}>
                <Trash2 size={15} />
                <span>{t("deleteLibrary")}</span>
              </Button>}
            </div>
          </Card>
        );
      })}
    </div>
  );
}
