import { useEffect, useState } from "react";
import { ChevronDown } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { TeamLibrary } from "../../api";
import type { TranslatorContext } from "../../types";
import { DialogShell } from "./dialog-shell";

export function UserLibraryAccessDialog({
  libraries,
  libraryRoles,
  open,
  subject,
  t,
  onClose,
  onSave
}: TranslatorContext & {
  libraries: TeamLibrary[];
  libraryRoles: Record<string, string>;
  open: boolean;
  subject?: string;
  onClose: () => void;
  onSave: (roles: Record<string, string>) => boolean | void | Promise<boolean | void>;
}) {
  const [draftRoles, setDraftRoles] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (open) {
      setDraftRoles(libraryRoles);
      setSaving(false);
    }
  }, [libraryRoles, open]);

  const setLibraryEnabled = (libraryId: string, enabled: boolean) => {
    setDraftRoles((current) => {
      const next = { ...current };
      if (enabled) next[libraryId] = current[libraryId] || "viewer";
      else delete next[libraryId];
      return next;
    });
  };

  const setLibraryRole = (libraryId: string, role: string) => {
    setDraftRoles((current) => ({ ...current, [libraryId]: role }));
  };

  const save = async () => {
    setSaving(true);
    try {
      const shouldClose = await onSave(draftRoles);
      if (shouldClose !== false) onClose();
    } finally {
      setSaving(false);
    }
  };

  return (
    <DialogShell
      className="user-library-access-dialog"
      closeLabel={t("cancel")}
      open={open}
      subtitle={subject ? `${subject} · ${t("manageLibraryAccessHint")}` : t("manageLibraryAccessHint")}
      title={t("manageLibraryAccess")}
      titleId="user-library-access-dialog-title"
      onClose={onClose}
    >
      <div className="user-library-access-dialog-body">
        {libraries.length === 0 ? (
          <div className="user-library-inherited">{t("noLibraries")}</div>
        ) : (
          <div className="user-library-access-list">
            {libraries.map((library) => {
              const enabled = Boolean(draftRoles[library.id]);
              const role = draftRoles[library.id] || "viewer";
              return (
                <div className="user-library-access-row" key={library.id}>
                  <label className="user-library-access-check">
                    <input
                      checked={enabled}
                      type="checkbox"
                      onChange={(event) => setLibraryEnabled(library.id, event.target.checked)}
                    />
                    <span>{library.displayName}</span>
                  </label>
                  <span className="user-library-access-role">
                    <select
                      aria-label={`${library.displayName} ${t("role")}`}
                      disabled={!enabled}
                      value={role}
                      onChange={(event) => setLibraryRole(library.id, event.target.value)}
                    >
                      {role === "owner" && <option value="owner">{t("owner")}</option>}
                      {role === "admin" && <option value="admin">{t("adminRole")}</option>}
                      <option value="library_manager">{t("manager")}</option>
                      <option value="editor">{t("editor")}</option>
                      <option value="viewer">{t("viewer")}</option>
                    </select>
                    <ChevronDown aria-hidden="true" size={14} />
                  </span>
                </div>
              );
            })}
          </div>
        )}
      </div>
      <div className="dialog-footer">
        <Button disabled={saving} type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
        <Button disabled={saving} type="button" onClick={() => void save()}>{t("save")}</Button>
      </div>
    </DialogShell>
  );
}
