/** The generated icons (`assets/icons/`, `packages/scene-assets/src/icons.ts`),
 *  drawn inline so they take the text colour around them. */
const SVG = import.meta.glob("../../../../assets/icons/**/*.svg", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

/** The icon at `path` under `assets/icons/` (e.g. `weapons/rifle.svg`); none
 *  when it is missing (the icons test fails first). */
export function Icon({ path, className = "ro-icon" }: { path: string; className?: string }) {
  const svg = SVG[`../../../../assets/icons/${path}`];
  return svg ? (
    <span className={className} aria-hidden="true" dangerouslySetInnerHTML={{ __html: svg }} />
  ) : null;
}
