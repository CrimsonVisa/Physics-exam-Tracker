import type { ReactNode } from 'react';

type ChartFrameProps = {
  title: string;
  description: string;
  columns: string[];
  rows: Array<Array<string | number>>;
  children: ReactNode;
  className?: string;
};

export function ChartFrame({
  title,
  description,
  columns,
  rows,
  children,
  className = '',
}: ChartFrameProps) {
  return (
    <figure className={`v2-chart-card ${className}`.trim()}>
      <figcaption>
        <h3>{title}</h3>
        <p>{description}</p>
      </figcaption>
      {children}
      {rows.length > 0 ? (
        <details className="v2-chart-data">
          <summary>View data table</summary>
          <div className="v2-chart-table-scroll">
            <table className="v2-chart-table">
              <thead>
                <tr>
                  {columns.map((column) => (
                    <th key={column} scope="col">
                      {column}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {rows.map((row, index) => (
                  <tr key={index}>
                    {row.map((cell, cellIndex) => (
                      <td key={cellIndex}>{cell}</td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </details>
      ) : null}
    </figure>
  );
}
