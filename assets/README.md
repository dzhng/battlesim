# assets

Art for the battle, owned by [`packages/scene-assets`](../packages/scene-assets/README.md).

- `catalog.json`: the authored catalog. It holds the skeletons and appearances with their sources and basis, the fit tolerances, and each side's tint. Text.
- `source/` and `third-party/`: GLBs and other inputs. Git LFS, whatever the extension. `source/infantry/` is exported by `packages/scene-assets/blender/`; re-export it there rather than editing the GLBs.
- `review/`: accepted model sheets (`asset sheet --accept`), one folder per appearance.
- `runtime/`: the bake's output. `<hash>/bundle.bin` is LFS, and `catalog.json` maps names to hashes. It is Vite's `publicDir`, so it is served at the site root and copied into production builds.

The `asset` CLI (`bun run --cwd web asset -- <command>`, source `web/asset.mjs`) owns every step on this folder; run it bare for its commands. The ones a model change always touches: `pull [name]` fetches exactly the LFS files an entry needs, `blender <script>` runs a script on the pinned Blender, `sheet <appearance|glb>` renders the workbench's review sheets headless into `throwaway/sheets/<name>/` (`--accept` copies them to `review/`), and `bake` then `check`.

The model workbench is `/workbench` in the lab app: drop a GLB, or open `/workbench?bundle=<name>`. While the dev server runs, a change under `source/` or to `catalog.json` re-bakes and reloads it.

After changing a source or the catalog, run `bake`, then `check`.
