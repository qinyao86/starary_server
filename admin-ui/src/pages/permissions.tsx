import { AppWindowMac, Server } from "lucide-react";
import type { TranslatorContext } from "../types";
import { libraryPermissions, serverPermissions } from "../mockData";
import { CheckCell, PageFrame, Panel } from "../components/common";

export function PermissionsPage({ t }: TranslatorContext) {
  const serverRoles = [
    { key: "owner", label: "serverOwnerRole" },
    { key: "admin", label: "serverAdminRole" },
    { key: "user", label: "standardUser" }
  ] as const;
  const clientRoles = [
    { key: "manager", label: "libraryManagerRole" },
    { key: "editor", label: "libraryEditorRole" },
    { key: "viewer", label: "libraryViewerRole" }
  ] as const;
  let currentCategory = "";

  return (
    <PageFrame title={t("permissions")} description={t("permissionsPageHint")}>
      <div className="permission-note">{t("libraryManagerConsoleNote")}</div>

      <Panel title={t("serverPermissions")} icon={Server} className="span-12">
        <table className="matrix permission-matrix">
          <thead>
            <tr>
              <th>{t("action")}</th>
              {serverRoles.map((role) => (
                <th key={role.key}>{t(role.label)}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {serverPermissions.map((row) => (
              <tr key={row.action}>
                <td>{t(row.action)}</td>
                <td><CheckCell checked={row.owner} /></td>
                <td><CheckCell checked={row.admin} /></td>
                <td><CheckCell checked={row.user} /></td>
              </tr>
            ))}
          </tbody>
        </table>
      </Panel>

      <Panel title={t("clientPermissions")} icon={AppWindowMac} className="span-12">
        <div className="permission-plan-note">{t("clientPermissionsPlanNote")}</div>
        <table className="matrix permission-matrix permission-client-matrix">
          <thead>
            <tr>
              <th>{t("action")}</th>
              {clientRoles.map((role) => (
                <th key={role.key}>{t(role.label)}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {libraryPermissions.flatMap((row) => {
              const categoryChanged = row.category !== currentCategory;
              currentCategory = row.category;
              return [
                ...(categoryChanged
                  ? [
                      <tr className="permission-category-row" key={row.category}>
                        <th colSpan={4}>{t(row.category)}</th>
                      </tr>
                    ]
                  : []),
                <tr className={row.verified ? "" : "permission-row-unverified"} key={`${row.action}:${row.scope}`}>
                  <td>
                    <div className="permission-action-cell">
                      <span
                        aria-label={t(row.verified ? "permissionVerified" : "permissionPendingVerification")}
                        className={`permission-verification-indicator ${row.verified ? "is-verified" : ""}`}
                        title={t(row.verified ? "permissionVerified" : "permissionPendingVerification")}
                      />
                      <span>
                        {"target" in row
                          ? t("permissionActionWithTarget")
                              .replace("{action}", t(row.action))
                              .replace("{target}", t(row.target))
                          : t(row.action)}
                      </span>
                    </div>
                  </td>
                  <td><CheckCell checked={row.manager} /></td>
                  <td><CheckCell checked={row.editor} /></td>
                  <td><CheckCell checked={row.viewer} /></td>
                </tr>
              ];
            })}
          </tbody>
        </table>
      </Panel>
    </PageFrame>
  );
}
