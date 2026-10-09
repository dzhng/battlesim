/** The generated icons (`assets/icons/`, `packages/scene-assets/src/icons.ts`),
 *  drawn inline so they take the text colour around them. */
const SVG = import.meta.glob("../../../../assets/icons/**/*.svg", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

/** Paths already reported missing: one report each, not one per frame. */
const reported = new Set<string>();

/** The icon at `path` under `assets/icons/` (e.g. `weapons/rifle.svg`), an
 *  `ro-icon` plus any `className`. A missing one draws nothing and says so:
 *  a path made at runtime (a weapon row's icon field) can name a file no
 *  static check sees, and a silent blank is the bug the player finds. */
export function Icon({
  path,
  className,
  title,
}: {
  path: string;
  className?: string;
  title?: string;
}) {
  const svg = SVG[`../../../../assets/icons/${path}`];
  if (!svg && !reported.has(path)) {
    reported.add(path);
    console.error(`missing icon: assets/icons/${path}`);
  }
  return svg ? (
    <span
      className={className ? `ro-icon ${className}` : "ro-icon"}
      title={title}
      aria-hidden="true"
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  ) : null;
}
