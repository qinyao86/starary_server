import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { ChevronDown } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { TeamUser } from "../../api";
import type { TranslatorContext } from "../../types";
import { DialogShell } from "./dialog-shell";

export function MemberDialog({
  open,
  t,
  users,
  memberSelections,
  onClose,
  onRoleChange,
  onSelectionChange,
  onSubmit,
}: TranslatorContext & {
  open: boolean;
  users: TeamUser[];
  memberSelections: Record<string, { checked: boolean; role: string }>;
  onClose: () => void;
  onRoleChange: (userId: string, role: string) => void;
  onSelectionChange: (userId: string, checked: boolean) => void;
  onSubmit: (event: FormEvent) => void | Promise<void>;
}) {
  const [query, setQuery] = useState("");
  const selectAllRef = useRef<HTMLInputElement>(null);
  const normalizedQuery = query.trim().toLowerCase();
  const filteredUsers = useMemo(() => {
    if (!normalizedQuery) return users;
    return users.filter((user) => (
      user.displayName.toLowerCase().includes(normalizedQuery)
      || user.email.toLowerCase().includes(normalizedQuery)
    ));
  }, [normalizedQuery, users]);
  const selectedCount = users.filter((user) => memberSelections[user.id]?.checked).length;
  const visibleSelectedCount = filteredUsers.filter((user) => memberSelections[user.id]?.checked).length;
  const allVisibleSelected = filteredUsers.length > 0 && visibleSelectedCount === filteredUsers.length;

  useEffect(() => {
    if (open) setQuery("");
  }, [open]);

  useEffect(() => {
    if (!selectAllRef.current) return;
    selectAllRef.current.indeterminate = visibleSelectedCount > 0 && visibleSelectedCount < filteredUsers.length;
  }, [filteredUsers.length, visibleSelectedCount]);

  const setAllVisibleSelected = (checked: boolean) => {
    filteredUsers.forEach((user) => onSelectionChange(user.id, checked));
  };

  return (
    <DialogShell
      className="member-dialog"
      closeLabel={t("cancel")}
      open={open}
      subtitle={t("addMemberHint")}
      title={t("addMember")}
      titleId="member-dialog-title"
      onClose={onClose}
    >
      <form className="dialog-form" onSubmit={onSubmit}>
        <div className="dialog-body">
          {users.length === 0 ? (
            <div className="placeholder-box">{t("noAvailableUsers")}</div>
          ) : (
            <>
              <label className="member-picker-search">
                <span>{t("searchUsers")}</span>
                <input
                  aria-label={t("searchUsers")}
                  placeholder={t("searchUsers")}
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                />
              </label>
              {filteredUsers.length === 0 ? (
                <div className="placeholder-box">{t("noSearchResults")}</div>
              ) : (
                <div className="member-picker-list">
                  {filteredUsers.map((user) => {
                    const selection = memberSelections[user.id] ?? { checked: false, role: "editor" };
                    return (
                      <div className={`member-picker-row${selection.checked ? " is-selected" : ""}`} key={user.id}>
                        <label className="member-picker-check">
                          <input
                            checked={selection.checked}
                            type="checkbox"
                            onChange={(event) => onSelectionChange(user.id, event.target.checked)}
                          />
                          <span>
                            <strong>{user.displayName}</strong>
                            <small>{user.email}</small>
                          </span>
                        </label>
                        <label className="member-role-control member-picker-role">
                          <select
                            aria-label={t("role")}
                            value={selection.role}
                            onChange={(event) => onRoleChange(user.id, event.target.value)}
                          >
                            <option value="library_manager">{t("manager")}</option>
                            <option value="editor">{t("editor")}</option>
                            <option value="viewer">{t("viewer")}</option>
                          </select>
                          <ChevronDown aria-hidden="true" size={14} />
                        </label>
                      </div>
                    );
                  })}
                </div>
              )}
            </>
          )}
        </div>
        <div className="dialog-footer">
          {users.length > 0 && (
            <label className="member-picker-select-all">
              <input
                ref={selectAllRef}
                checked={allVisibleSelected}
                disabled={filteredUsers.length === 0}
                type="checkbox"
                onChange={(event) => setAllVisibleSelected(event.target.checked)}
              />
              <span>{t("selectAll")}</span>
            </label>
          )}
          <Button type="button" variant="outline" onClick={onClose}>{t("cancel")}</Button>
          <Button type="submit" disabled={selectedCount === 0}>{t("submit")}</Button>
        </div>
      </form>
    </DialogShell>
  );
}
