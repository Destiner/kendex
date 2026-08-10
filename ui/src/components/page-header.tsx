export function PageHeader({
  title,
  subtitle,
}: {
  title: string;
  subtitle?: string;
}) {
  return (
    <header className="border-b px-8 py-5">
      <h1 className="text-lg font-semibold tracking-tight">{title}</h1>
      {subtitle ? (
        <p className="mt-0.5 text-sm text-muted-foreground">{subtitle}</p>
      ) : null}
    </header>
  );
}
