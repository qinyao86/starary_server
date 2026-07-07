export function LibraryStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="library-card-stat">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

export function LibraryDetailStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="library-detail-stat">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
