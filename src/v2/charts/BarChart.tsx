import { CHART_COLORS, truncateLabel } from './chartMath';

type BarDatum = {
  label: string;
  value: number;
  valueLabel?: string;
};

type BarChartProps = {
  ariaLabel: string;
  data: BarDatum[];
  maxValue?: number;
  valueSuffix?: string;
};

export function BarChart({ ariaLabel, data, maxValue, valueSuffix = '' }: BarChartProps) {
  if (data.length === 0) {
    return <p className="v2-empty-panel-copy">No chartable data in this range.</p>;
  }
  const width = 560;
  const labelWidth = 158;
  const barWidth = 300;
  const rowHeight = 30;
  const height = data.length * rowHeight + 10;
  const max = Math.max(1, maxValue ?? Math.max(...data.map((entry) => entry.value), 0));

  return (
    <svg
      aria-label={ariaLabel}
      className="v2-bar-chart"
      role="img"
      viewBox={`0 0 ${width} ${height}`}
    >
      <title>{ariaLabel}</title>
      {data.map((entry, index) => {
        const y = index * rowHeight + 8;
        const widthValue = Math.max(0, Math.min(1, entry.value / max)) * barWidth;
        return (
          <g key={entry.label}>
            <text className="v2-chart-axis-label" x="0" y={y + 12}>
              {truncateLabel(entry.label, 24)}
            </text>
            <rect
              className="v2-bar-track"
              height="14"
              rx="4"
              width={barWidth}
              x={labelWidth}
              y={y + 2}
            />
            <rect
              fill={CHART_COLORS[index % CHART_COLORS.length]}
              height="14"
              rx="4"
              width={widthValue}
              x={labelWidth}
              y={y + 2}
            />
            <text className="v2-chart-value-label" x={labelWidth + barWidth + 8} y={y + 13}>
              {entry.valueLabel ?? `${entry.value}${valueSuffix}`}
            </text>
          </g>
        );
      })}
    </svg>
  );
}
