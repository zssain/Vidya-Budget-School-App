export function DataTable({ columns, rows, rowKey = 'id', onRow }) {
  return (
    <div className="table-wrap">
      <table>
        <thead>
          <tr>
            {columns.map((column) => (
              <th key={column.key}>{column.label}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, index) => (
            <tr
              key={row[rowKey] ?? index}
              tabIndex={onRow ? 0 : undefined}
              onClick={() => onRow?.(row)}
              onKeyDown={(event) => {
                if (onRow && (event.key === 'Enter' || event.key === ' ')) onRow(row);
              }}
            >
              {columns.map((column) => (
                <td key={column.key}>{column.render ? column.render(row) : row[column.key]}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
