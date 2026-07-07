import { ShieldCheck } from "lucide-react";
import type { TranslatorContext } from "../types";
import { permissions } from "../mockData";
import { Badge, CheckCell, Panel } from "../components/common";

export function PermissionsPage({ t }: TranslatorContext) {
  const roles = ["owner", "adminRole", "manager", "editor", "viewer"] as const;
  return (
    <div className="page-grid">
      <Panel title={t("permissionMatrix")} icon={ShieldCheck} className="span-12" action={<Badge>{t("placeholderData")}</Badge>}>
        <table className="matrix">
          <thead>
            <tr>
              <th>{t("action")}</th>
              {roles.map((role) => (
                <th key={role}>{t(role)}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {permissions.map((row) => (
              <tr key={row.action}>
                <td>{t(row.action)}</td>
                <td><CheckCell checked={row.owner} /></td>
                <td><CheckCell checked={row.admin} /></td>
                <td><CheckCell checked={row.manager} /></td>
                <td><CheckCell checked={row.editor} /></td>
                <td><CheckCell checked={row.viewer} /></td>
              </tr>
            ))}
          </tbody>
        </table>
      </Panel>
    </div>
  );
}
