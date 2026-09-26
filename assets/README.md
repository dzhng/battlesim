# assets

Art for the battle, owned by [`packages/scene-assets`](../packages/scene-assets/README.md).

- `catalog.json`: the authored catalog. It holds the skeletons and appearances with their sources and basis, the fit tolerances, and each side's tint. Text.
- `source/` and `third-party/`: GLBs and other inputs. Git LFS, whatever the extension. `source/infantry/` is exported by `packages/scene-assets/blender/`; re-export it there rather than editing the GLBs.
- `review/`: accepted model sheets (`asset sheet --accept`), one folder per appearance.
- `runtime/`: the bake's output. `<hash>/bundle.bin` is LFS, and `catalog.json` maps names to hashes. It is Vite's `publicDir`, so it is served at the site root and copied into production builds.

`bun run --cwd web asset -- <command>`:
- `validate <glb>` prints stats and findings;
- `bake` rewrites `runtime/`;
- `check` fails when `runtime/` is stale;
- `provenance <file>` shows a file's hash and manifest entry;
- `pull [name]` fetches exactly the LFS files an entry needs;
- `grass [name]` regenerates the grass kinds' GLBs (`source/grass/`) from their catalog specs and records their hashes;
- `blender <script>` runs a script on the pinned Blender 5.2.1;
- `sheet <appearance|glb>` renders the model workbench's contact sheet, strips, stats, impostor atlas, surface sheet (close and battle views, each texture channel off in turn) and texture preview headless, into `throwaway/sheets/<name>/` (`--accept` copies them to `review/`).

The model workbench is `/workbench` in the lab app: drop a GLB, or open `/workbench?bundle=<name>`. While the dev server runs, a change under `source/` or to `catalog.json` re-bakes and reloads it.

After changing a source or the catalog, run `bake`, then `check`.
