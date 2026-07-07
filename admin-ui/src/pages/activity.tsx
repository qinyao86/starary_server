import { ListChecks } from "lucide-react";
import type { ActivityItem } from "../api";
import type { TranslatorContext } from "../types";
import { ActivityList, Badge, Panel } from "../components/common";

export function ActivityPage({ t, activityItems }: TranslatorContext & { activityItems: ActivityItem[] }) {
  return (
    <div className="page-grid">
      <Panel title={t("activity")} icon={ListChecks} className="span-12" action={<Badge>{activityItems.length ? t("realData") : t("placeholderData")}</Badge>}>
        <ActivityList t={t} activityItems={activityItems} />
      </Panel>
    </div>
  );
}
