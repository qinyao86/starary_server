import type { ActivityItem } from "../../api";
import type { TranslatorContext } from "../../types";
import { activityActionLabel } from "../../utils/format";
import { UserAvatar } from "./user-avatar";
import { StatusDot } from "./status";

export function ActivityList({
  t,
  activityItems,
  compact = false,
  resolveAvatarKey
}: TranslatorContext & {
  activityItems: ActivityItem[];
  compact?: boolean;
  resolveAvatarKey?: (item: ActivityItem) => string | null | undefined;
}) {
  return ActivityListWithAvatarLookup({ t, activityItems, compact, resolveAvatarKey });
}

export function ActivityListWithAvatarLookup({
  t,
  activityItems,
  compact = false,
  resolveAvatarKey
}: TranslatorContext & {
  activityItems: ActivityItem[];
  compact?: boolean;
  resolveAvatarKey?: (item: ActivityItem) => string | null | undefined;
}) {
  const rows = activityItems.map((item) => ({
    actor: item.actorDisplayName ?? item.actorEmail ?? item.actorUserId ?? t("system"),
    avatarKey: resolveAvatarKey?.(item) ?? item.actorAvatarKey ?? null,
    action: activityActionLabel(t, item.action),
    target: item.targetName ?? item.targetType ?? t("unknownTarget"),
    time: new Date(item.createdAt).toLocaleString()
  }));

  return (
    <div className={`activity-list${compact ? " is-compact" : ""}`}>
      {rows.length === 0 ? (
        <div className="activity-empty">{t("empty")}</div>
      ) : (
        rows.map((item) => (
          <div className="activity-item" key={`${item.actor}-${item.time}-${item.action}`}>
            <UserAvatar avatarKey={item.avatarKey} label={item.actor} size="md" />
            <div className="activity-main">
              <div className="activity-action">{item.action}</div>
              <div className="activity-meta">
                {item.actor} / {item.target}
              </div>
            </div>
            <div className="activity-time">{item.time}</div>
          </div>
        ))
      )}
      {!compact && (
        <div className="activity-footer">
          <StatusDot label={activityItems.length ? t("realData") : t("empty")} tone={activityItems.length ? "good" : "muted"} />
        </div>
      )}
    </div>
  );
}
