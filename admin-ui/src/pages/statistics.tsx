import { BarChart3, ListChecks, Network, RefreshCw } from "lucide-react";
import type { TeamLibrary, TeamUser } from "../api";
import type { TranslatorContext } from "../types";
import { assetBreakdown } from "../mockData";
import { Badge, BarList, InfoStack, Panel, TrendBars } from "../components/common";

export function StatisticsPage({ t, assetTotal, users, libraries }: TranslatorContext & { assetTotal: number; users: TeamUser[]; libraries: TeamLibrary[] }) {
  return (
    <div className="page-grid">
      <Panel title={t("assetTypes")} icon={BarChart3} className="span-6" action={<Badge>{t("placeholderData")}</Badge>}>
        <BarList t={t} items={assetBreakdown} />
      </Panel>
      <Panel title={t("imports")} icon={Network} className="span-6" action={<Badge>{t("placeholderData")}</Badge>}>
        <TrendBars />
      </Panel>
      <Panel title={t("statistics")} icon={RefreshCw} className="span-6" action={<Badge>{t("realData")}</Badge>}>
        <InfoStack items={[[t("totalAssets"), String(assetTotal)], [t("users"), String(users.length)], [t("libraries"), String(libraries.length)]]} />
      </Panel>
      <Panel title={t("auditEvents")} icon={ListChecks} className="span-6" action={<Badge>{t("placeholderData")}</Badge>}>
        <InfoStack items={[[t("activity"), t("plannedNote")], [t("retention"), "365 days"], [t("status"), t("healthy")]]} />
      </Panel>
    </div>
  );
}
