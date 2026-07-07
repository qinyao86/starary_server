import { Library, Pencil, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import type { TeamLibrary } from "../../api";
import type { TranslatorContext } from "../../types";
import { formatBytes, formatCount } from "../../utils/format";
import { LibraryStat } from "../common";

export function LibraryCardList({
  libraries,
  t,
  onDelete,
  onEdit,
  onOpen
}: TranslatorContext & {
  libraries: TeamLibrary[];
  onDelete: (library: TeamLibrary) => void;
  onEdit: (library: TeamLibrary) => void;
  onOpen: (libraryId: string) => void;
}) {
  if (libraries.length === 0) {
    return <div className="placeholder-box">{t("noLibraries")}</div>;
  }

  return (
    <div className="library-card-grid">
      {libraries.map((library) => (
        <Card className="library-card" key={library.id}>
          <div className="library-card-top">
            <div className="library-card-heading">
              <div className="library-card-title">{library.displayName}</div>
              <div className="library-card-description">{library.description || t("noDescription")}</div>
            </div>
            <div className="library-card-tools">
              <button className="library-icon-button" type="button" aria-label={t("editLibrary")} onClick={() => onEdit(library)}>
                <Pencil size={15} />
              </button>
              <button className="library-icon-button is-danger" type="button" aria-label={t("deleteLibrary")} onClick={() => onDelete(library)}>
                <Trash2 size={15} />
              </button>
            </div>
          </div>
          <div className="library-card-stats">
            <LibraryStat label={t("totalSize")} value={formatBytes(library.totalSizeBytes)} />
            <LibraryStat label={t("membersLabel")} value={formatCount(library.memberNames?.length)} />
            <LibraryStat label={t("assets")} value={formatCount(library.assetCount)} />
            <LibraryStat label={t("tags")} value={formatCount(library.tagCount)} />
          </div>
          <div className="library-card-footer">
            <div className="library-card-members" title={library.creatorName ?? ""}>
              <span>{t("creator")}</span>
              <strong>{library.creatorName || "-"}</strong>
            </div>
            <Button size="sm" type="button" onClick={() => onOpen(library.id)}>
              <Library size={15} />
              <span>{t("openLibrary")}</span>
            </Button>
          </div>
        </Card>
      ))}
    </div>
  );
}
