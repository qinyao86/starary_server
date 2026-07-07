import type { LucideIcon } from "lucide-react";

export function StatusDot({ label, tone }: { label: string; tone: "good" | "warn" | "muted" }) {
  return (
    <div className={`status-dot tone-${tone}`}>
      <span />
      {label}
    </div>
  );
}

export function HealthRow({
  icon: Icon,
  label,
  value,
  tone
}: {
  icon: LucideIcon;
  label: string;
  value: string;
  tone: "good" | "warn" | "muted";
}) {
  return (
    <div className="health-row">
      <div className="health-icon"><Icon size={17} /></div>
      <div>
        <div className="health-label">{label}</div>
        <StatusDot label={value} tone={tone} />
      </div>
    </div>
  );
}
