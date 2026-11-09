import React from 'react';
type Props = { title: string; items: string[] };
export const List: React.FC<Props> = ({title,items}) => {
  const rows = items.map((item, index) =>
    <li key={index} className="row">
      {item}
    </li>
  );
  return (
    <section>
      <h1 title="multi
  line attribute">{title}</h1>
      {rows.length > 0 ? <ul>{rows}</ul> : <p>Nothing   here</p>}
      {`template
   literal`}
    </section>
  );
};
function generic<T,>(value: T): T { return value; }
