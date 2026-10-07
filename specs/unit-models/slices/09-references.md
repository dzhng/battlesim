# 09 References

**Unlocks:** photos for every family, committed, so modelling copies what the
vehicle looks like rather than what a spec table says.

## Work

For every family in the README's [scope table](../README.md#scope), including
the disabled cards (slice 15 uses them):

1. Find photos of the exact variant (SEPv3 is not SEPv2; BMP-3 is not BMP-2M).
   Coverage per variant: `three_quarter_front`, `side`, `three_quarter_rear`,
   plus `front`, `rear`, `top` and `detail` (running gear, turret roof, sights,
   stowage) where they exist. For infantry: uniform, helmet, carrier, and each
   team weapon deployed and carried.
2. **Licence.** Only images we may redistribute: public domain (US DoD and
   DVIDS photos, most US Army/USMC releases), CC0, CC BY, CC BY-SA. Wikimedia
   Commons file pages state this; record the file page, not a thumbnail URL.
   No press, manufacturer-brochure or game images unless their licence says
   so. **Where photos are scarce** (user chose, 2026-10-06; likely J-20,
   Type 15, KF51, Tornado-S, parts of the Chinese and Russian air and air
   defence lists): use whatever licensable photos exist, even few or small,
   and licensable line drawings or three-views (Commons has many).
2b. **Generated views fill the gaps.** For a view no licensable image covers,
   generate one with the latest gpt-image model through the duet CLI,
   conditioned on the family's real references:
   `duet model -m openai/gpt-image-<latest> --image <real reference> --size 1536x1024 -o <file> "<variant>, <view>, …"`
   (check `duet model --help` and the gateway catalog for the current model id;
   `DUET_API_KEY` must be set). A generated image is labelled everywhere: file
   name suffix `-generated`, `references.json` entry with
   `"source": "generated"`, the model id, the full prompt, the input
   reference files and the seed; licence is ours. It shows a view's layout
   and proportions; it is never evidence of a detail a real photo
   contradicts or doesn't show, and the review sheet marks it as generated.
   Record what each family's model had to guess in its `gaps` and its notes.
3. **Store.** `assets/references/<family>/<variant>-<view>[-n].jpg`, resized
   so the long edge is at most 1600 px (smaller originals stay as they are),
   re-encoded JPEG at a quality that keeps panel lines legible (LFS). Beside them `references.json`:
   `[{file, variant, view, source (photo | drawing | generated), page, author, licence, sha256, note}]` (generated entries add model, prompt, inputs, seed), and a
   `gaps` list. Add a line to [assets README](../../../assets/README.md)'s
   third-party table pointing at `assets/references/` and stating these are
   review inputs, never shipped.
4. Record in each family's file what the photos settle that the old manifest
   got wrong (wheel counts, hatch layout, sight positions). A disagreement with
   a physical frame goes to choices.md for the user; it does not move the frame.

## Parallelism

Independent per family; split across workers by the README's groups. Each
worker owns only its families' folders.

## Verify

A script check (in the asset CLI's `check` or a test): every
`references.json` parses, every file exists, its sha256 matches, and every
entry has a licence from the allowed list. Look at each family's set once.

## Delegated

Which photos, as long as coverage and licence hold.
