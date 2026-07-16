import type { LucideIcon } from "lucide-react";

export function LibraryStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="library-card-stat">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

export function LibraryDetailStat({ icon: Icon, label, value }: { icon: LucideIcon; label: string; value: string }) {
  return (
    <div className="library-detail-stat">
      <div className="library-detail-stat-icon">
        <Icon aria-hidden="true" size={18} />
      </div>
      <div className="library-detail-stat-copy">
        <span>{label}</span>
        <strong>{value}</strong>
      </div>
    </div>
  );
}
