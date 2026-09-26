// The battle's appearances: the baked runtime catalog (Vite's public
// directory, served at the site root), loaded once through the one loader.
// The battle frame draws its grass kinds from it today, and its soldiers and
// vehicles once slices 23 and 24 land.
import { AppearanceLibrary, type InstalledAppearances } from "@packages/scene-assets/src/loader";

let installed: Promise<InstalledAppearances> | null = null;

export function battleAppearances(): Promise<InstalledAppearances> {
  installed ??= new AppearanceLibrary().load(import.meta.env.BASE_URL);
  return installed;
}
