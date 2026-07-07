import type { TranslationKey } from "../../i18n";
import type { TranslatorContext } from "../../types";

export function MetricCard({ label, value, change, tone }: { label: string; value: string; change: string; tone: string }) {
  return (
    <div className={`metric-card tone-${tone}`}>
      <div className="metric-label">{label}</div>
      <div className="metric-row">
        <span className="metric-value">{value}</span>
        <span className="metric-change">{change}</span>
      </div>
    </div>
  );
}

export function BarList({ t, items }: TranslatorContext & { items: ReadonlyArray<{ label: TranslationKey; value: number }> }) {
  return (
    <div className="bar-list">
      {items.map((item) => (
        <div className="bar-item" key={item.label}>
          <div className="bar-meta">
            <span>{t(item.label)}</span>
            <strong>{item.value}%</strong>
          </div>
          <div className="bar-track">
            <span style={{ width: `${item.value}%` }} />
          </div>
        </div>
      ))}
    </div>
  );
}

export function TrendBars() {
  const values = [24, 36, 28, 54, 48, 62, 74, 58, 69, 82, 77, 88];
  return (
    <div className="trend-bars">
      {values.map((value, index) => (
        <span key={`${value}-${index}`} style={{ height: `${value}%` }} />
      ))}
    </div>
  );
}
