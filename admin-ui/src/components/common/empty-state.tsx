export function EmptyState({ illustration, label }: { illustration: string; label: string }) {
  return (
    <div className="management-empty-state">
      <img alt="" aria-hidden="true" src={illustration} />
      <strong>{label}</strong>
    </div>
  );
}
