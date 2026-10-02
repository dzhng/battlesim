# procedural-buildings (vendored)

Three procedural buildings made with Blender geometry nodes. They are the source of truth for the city kit's New York, Paris and China archetypes: the building scripts in [`../../city/`](../../city/) open them headless, patch their inputs and export kit modules and template placements. Nothing here is edited by hand.

| File                         | What it is                                                            |
| ---------------------------- | --------------------------------------------------------------------- |
| `NYC_CornerBuilding.blend`   | A pre-war New York corner building                                    |
| `FrenchBuilding.blend`       | A Haussmann-style Paris block                                         |
| `CN_ApartmentBuilding.blend` | A Chinese corner apartment building with shops and enclosed balconies |

**Source:** <https://github.com/achrefelouafi/ProceduralBuildingsThreeJS>, `blender/`, at commit `6c19f1b1f14408353989ad26cda27f1e1ede5e91`, copied unchanged.

**Licence:** MIT, © 2026 mohamedachrefelouafi. The notice is [`LICENSE`](LICENSE) beside the files.

Each file embeds its author's own generator scripts and no linked or purchased library. They were saved in Blender 5.1 and are opened in the pinned 5.2.1, where a modifier input is `modifier.properties.inputs.Socket_N.value`.

## Not vendored

- **The textures.** The files' materials read ambientCG sets (CC0). Those are fetched into the local pack cache and pinned by hash in [`../../packs.json`](../../packs.json); none is committed.
- **The two interior photo atlases** (`textures/misc/apartmentinterios.png`, `businesses.png`). Their source is not stated, so they never enter this repo. Interiors use our own generated atlas.
- **The street trees.** Street trees are ours (`../../trees.py`).
