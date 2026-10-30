type ChartDatum = {
  key: string;
  label: string;
  title: string;
  value: number;
};

type MetricBarChartProps = {
  data: ChartDatum[];
  ariaLabel: string;
  formatValue: (value: number) => string;
  referenceValue?: number;
  referenceLabel?: string;
  emptyMessage: string;
};

function visibleLabel(index: number, length: number, label: string): string | null {
  if (length <= 10) {
    return label;
  }
  const stride = Math.ceil(length / 8);
  return index % stride === 0 || index === length - 1 ? label : null;
}

export function MetricBarChart({
  data,
  ariaLabel,
  formatValue,
  referenceValue,
  referenceLabel,
  emptyMessage,
}: MetricBarChartProps) {
  if (data.length === 0) {
    return <p className="analytics-empty">{emptyMessage}</p>;
  }

  const maximum = Math.max(1, ...data.map((datum) => datum.value), referenceValue ?? 0);
  const referencePercent =
    referenceValue !== undefined && referenceValue > 0
      ? Math.min(100, (referenceValue / maximum) * 100)
      : null;

  return (
    <figure className="analytics-chart">
      <div className="analytics-chart-frame">
        <div className="analytics-chart-axis" aria-hidden="true">
          <span>{formatValue(maximum)}</span>
          <span>0</span>
        </div>
        <div className="analytics-chart-plot" role="img" aria-label={ariaLabel}>
          {referencePercent !== null && (
            <div
              className="analytics-chart-reference"
              style={{ bottom: `${referencePercent}%` }}
              aria-hidden="true"
            >
              <span>{referenceLabel}</span>
            </div>
          )}
          {data.map((datum, index) => {
            const height = Math.max(0, Math.min(100, (datum.value / maximum) * 100));
            const label = visibleLabel(index, data.length, datum.label);
            return (
              <div className="analytics-chart-column" key={datum.key} title={datum.title}>
                <div
                  className={`analytics-chart-bar${datum.value === 0 ? ' analytics-chart-bar-empty' : ''}`}
                  style={{ height: `${height}%` }}
                />
                <span className="analytics-chart-x-label">{label ?? '\u00a0'}</span>
              </div>
            );
          })}
        </div>
      </div>
      <figcaption className="analytics-chart-caption">
        {referencePercent !== null
          ? `${referenceLabel ?? 'Reference'} is shown as a horizontal guide. `
          : ''}
        Values are also available in the supporting-data table.
      </figcaption>
      <details className="analytics-chart-values">
        <summary>View supporting values</summary>
        <div className="analytics-table-wrap">
          <table className="analytics-table analytics-table-compact">
            <thead>
              <tr>
                <th scope="col">Period</th>
                <th scope="col">Value</th>
              </tr>
            </thead>
            <tbody>
              {data.map((datum) => (
                <tr key={`value-${datum.key}`}>
                  <th scope="row">{datum.title}</th>
                  <td>{formatValue(datum.value)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </details>
    </figure>
  );
}

type HorizontalBarListProps = {
  data: ChartDatum[];
  formatValue: (value: number) => string;
  emptyMessage: string;
  ariaLabel: string;
};

export function HorizontalBarList({
  data,
  formatValue,
  emptyMessage,
  ariaLabel,
}: HorizontalBarListProps) {
  if (data.length === 0) {
    return <p className="analytics-empty">{emptyMessage}</p>;
  }

  const maximum = Math.max(1, ...data.map((datum) => datum.value));
  return (
    <div className="analytics-hbars" role="img" aria-label={ariaLabel}>
      {data.map((datum) => (
        <div className="analytics-hbar-row" key={datum.key} title={datum.title}>
          <span className="analytics-hbar-label">{datum.label}</span>
          <div className="analytics-hbar-track" aria-hidden="true">
            <span
              className="analytics-hbar-fill"
              style={{ width: `${Math.max(0, Math.min(100, (datum.value / maximum) * 100))}%` }}
            />
          </div>
          <span className="analytics-hbar-value">{formatValue(datum.value)}</span>
        </div>
      ))}
    </div>
  );
}
