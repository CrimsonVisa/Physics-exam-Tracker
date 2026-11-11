import { CHART_COLORS, formatChartPercent } from './chartMath';

type DonutDatum = {
  label: string;
  value: number;
  valueLabel?: string;
};

type DonutChartProps = {
  ariaLabel: string;
  data: DonutDatum[];
};

export function DonutChart({ ariaLabel, data }: DonutChartProps) {
  const visible = data.filter((entry) => entry.value > 0);
  const total = visible.reduce((sum, entry) => sum + entry.value, 0);
  if (total <= 0) {
    return <p className="v2-empty-panel-copy">No chartable data in this range.</p>;
  }
  const radius = 44;
  const circumference = 2 * Math.PI * radius;
  const segments = visible.map((entry, index) => {
    const preceding = visible.slice(0, index).reduce((sum, item) => sum + item.value, 0);
    return {
      ...entry,
      dash: (entry.value / total) * circumference,
      dashOffset: -(preceding / total) * circumference,
    };
  });

  return (
    <div className="v2-donut-layout">
      <svg aria-label={ariaLabel} className="v2-donut" role="img" viewBox="0 0 120 120">
        <title>{ariaLabel}</title>
        <circle
          className="v2-donut-track"
          cx="60"
          cy="60"
          fill="none"
          r={radius}
          strokeWidth="14"
        />
        {segments.map((entry, index) => {
          return (
            <circle
              cx="60"
              cy="60"
              fill="none"
              key={entry.label}
              r={radius}
              stroke={CHART_COLORS[index % CHART_COLORS.length]}
              strokeDasharray={`${entry.dash} ${circumference - entry.dash}`}
              strokeDashoffset={entry.dashOffset}
              strokeWidth="14"
              transform="rotate(-90 60 60)"
            />
          );
        })}
        <text className="v2-donut-total" textAnchor="middle" x="60" y="58">
          {visible.length}
        </text>
        <text className="v2-donut-caption" textAnchor="middle" x="60" y="74">
          categories
        </text>
      </svg>
      <ul className="v2-chart-legend">
        {visible.map((entry, index) => (
          <li key={entry.label}>
            <span
              aria-hidden="true"
              className="v2-chart-swatch"
              style={{ background: CHART_COLORS[index % CHART_COLORS.length] }}
            />
            <span>{entry.label}</span>
            <strong>{entry.valueLabel ?? formatChartPercent((entry.value / total) * 100)}</strong>
          </li>
        ))}
      </ul>
    </div>
  );
}
