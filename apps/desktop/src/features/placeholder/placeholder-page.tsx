interface PlaceholderPageProps { title: string; description: string; path: string }

export function PlaceholderPage({ title, description, path }: PlaceholderPageProps) {
  return <div><header className="page-heading"><div><p className="path-label">C:\ {path}</p><h1>{title}</h1><p>{description}</p></div></header><section className="panel placeholder-panel"><strong>Módulo preparado para expansão</strong><p>A navegação, o modelo de permissões e a camada de dados já reservam este domínio sem misturar responsabilidades.</p></section></div>;
}
