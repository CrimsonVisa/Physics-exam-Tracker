import { CHART_COLORS, shortChartDate } from './chartMath';

type LinePoint = {
  x: string;
  y: number;
};

type LineSeries = {
  label: string;
  points: LinePoint[];
};

type LineChartProps = {
  ariaLabel: string;
  series: LineSeries[];
  valueSuffix?: string;
  formatValue?: (value: number) => string;
};

export function LineChart({ ariaLabel, series, valueSuffix = '', formatValue }: LineChartProps) {
  const allPoints = series.flatMap((entry) => entry.points);
  if (allPoints.length === 0) {
    return <p className="v2-empty-panel-copy">No chartable data in this range.</p>;
  }
  const xValues = [...new Set(allPoints.map((point) => point.x))].sort();
  const yValues = allPoints.map((point) => point.y);
  const dataMin = Math.min(...yValues);
  const dataMax = Math.max(...yValues);
  const yMin = Math.min(0, dataMin);
  const yMax = Math.max(0, dataMax);
  const yPadding = Math.max(1, (yMax - yMin) * 0.08);
  const low = yMin - (yMin < 0 ? yPadding : 0);
  const high = yMax + yPadding;
  const width = 680;
  const height = 270;
  const left = 52;
  const right = 18;
  const top = 16;
  const bottom = 34;
  const plotWidth = width - left - right;
  const plotHeight = height - top - bottom;
  const xFor = (value: string) => {
    const index = xValues.indexOf(value);
    return (
      left + (xValues.length <= 1 ? plotWidth / 2 : (index / (xValues.length - 1)) * plotWidth)
    );
  };
  const yFor = (value: number) => top + ((high - value) / (high - low || 1)) * plotHeight;
  const ticks = Array.from({ length: 5 }, (_, index) => low + ((high - low) * index) / 4);
  const xLabels =
    xValues.length <= 3
      ? xValues
      : [xValues[0], xValues[Math.floor((xValues.length - 1) / 2)], xValues[xValues.length - 1]];

  return (
    <div className="v2-line-chart-wrap">
      <svg
        aria-label={ariaLabel}
        className="v2-line-chart"
        role="img"
        viewBox={`0 0 ${width} ${height}`}
      >
        <title>{ariaLabel}</title>
        {ticks.map((tick) => (
          <g key={tick}>
            <line
              className="v2-chart-gridline"
              x1={left}
              x2={width - right}
              y1={yFor(tick)}
              y2={yFor(tick)}
            />
            <text className="v2-chart-axis-label" textAnchor="end" x={left - 8} y={yFor(tick) + 4}>
              {formatValue ? formatValue(tick) : `${Math.round(tick)}${valueSuffix}`}
            </text>
          </g>
        ))}
        <line className="v2-chart-zero" x1={left} x2={width - right} y1={yFor(0)} y2={yFor(0)} />
        {series.map((entry, index) => {
          const color = CHART_COLORS[index % CHART_COLORS.length];
          const points = [...entry.points].sort((leftPoint, rightPoint) =>
            leftPoint.x.localeCompare(rightPoint.x),
          );
          const path = points.map((point) => `${xFor(point.x)},${yFor(point.y)}`).join(' ');
          return (
            <g key={entry.label}>
              <polyline
                fill="none"
                points={path}
                stroke={color}
                strokeLinejoin="round"
                strokeWidth="2.5"
              />
              {points.map((point) => (
                <circle
                  cx={xFor(point.x)}
                  cy={yFor(point.y)}
                  fill={color}
                  key={`${entry.label}-${point.x}`}
                  r="3.2"
                >
                  <title>
                    {entry.label}: {shortChartDate(point.x)} ·{' '}
                    {formatValue ? formatValue(point.y) : `${point.y}${valueSuffix}`}
                  </title>
                </circle>
              ))}
            </g>
          );
        })}
        {xLabels.map((label) => (
          <text
            className="v2-chart-axis-label"
            key={label}
            textAnchor="middle"
            x={xFor(label)}
            y={height - 8}
          >
            {shortChartDate(label)}
          </text>
        ))}
      </svg>
      <ul className="v2-chart-legend v2-chart-legend-inline">
        {series.map((entry, index) => (
          <li key={entry.label}>
            <span
              aria-hidden="true"
              className="v2-chart-swatch"
              style={{ background: CHART_COLORS[index % CHART_COLORS.length] }}
            />
            <span>{entry.label}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
