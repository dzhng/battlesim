# 10 Materials

**Unlocks:** black tyres, dark glass and each nation's paint on every current
model, before any geometry changes, with a guard so they stay that way.

**Slice variable:** surface colour and finish only, toward the [look target](../assets/look-target.png): moderate weathering and painted edge highlights (wear chips on convex edges in a lighter tone of the scheme), not heavy grime. Shape is slices 12–15.

## Work

1. **Test first** (`write-tests`). In `scene-assets` validation, a material
   whose glTF `extras.role` is `rubber` fails (`material.role_rubber`) when its
   drawn albedo (albedo texture mean times mean vertex colour, as
   `src/material.ts` interprets them) exceeds a luminance bound; `glass` fails
   above a luminance bound or below a smoothness bound. Write the failing test
   with a GLB fixture built like today's roster tyre (rubber recipe, dust
   film), then make it pass. The bounds are chosen from what black rubber and
   sight glass look like on screen; record them in choices.md.
2. **Roles.** `parts.py` material helpers take `role=` and write it to extras.
   Add named helpers every exporter uses instead of ad-hoc `textured(...)`:
   `tyre()` (rubber recipe, dust capped so the tyre stays black with a dusty
   tread, not a grey one), `glass()` (near-black, low roughness, faint blue-green
   only at grazing angles via roughness, not albedo), `track_steel()`,
   `bare_steel()`, `paint(scheme)`.
   A string `extras.role` does not reach the validator today: material extras
   are read by key in `material.ts`, and node extras keep numbers only
   (`build.ts:697-699`). Carry the role through the build. Glass stays
   **opaque** with `chip=0`: `material.wear` refuses a blended surface that
   wears, and `textured()` defaults to `chip=0.8`.
3. **Schemes** (user chose, 2026-10-06): each vehicle wears its real
   nation's scheme, US in desert tan. One recipe per scheme in `textures.py`,
   each a seamless tile at metre scale:

   | Scheme | Families |
   |---|---|
   | US desert tan (CARC 686A) | Abrams, Bradley, Stryker, LAV, ACV, HMMWV, HEMTT |
   | German three-tone (NATO green/brown/black) | Leopard 2, Puma, KF51, Boxer, Fennek, MAN HX |
   | French three-tone | Leclerc, VBCI, VBL |
   | British green | Challenger 2, Ajax |
   | Swedish (CV90's origin) | CV90 |
   | Russian protective green (three-tone where a photo shows it) | T-72B3, T-80BVM, T-90M, BMP-2M, BMP-3, BTR-82A, Tigr-M, Ural |
   | Chinese digital woodland | Type 99A, ZBL-08 |

   Leopard 2 and Leclerc already carry a camo print: keep it if their
   references agree. The scheme is data: a `scheme` field in each family's
   exporter table. Disabled cards (slice 17) follow the same rule. Side tint
   keeps working through the ORM alpha mask on paint only, never on rubber,
   glass or steel; red's warm multiplier applies on top of the real scheme.
4. **Apply** to every runtime vehicle exporter as it stands (one-liner branches
   included): swap materials only, no geometry. Re-export, bake, check.
   Optics boxes become `glass()` in a dark housing colour.

## Verify

- The role test, red then green.
- `asset sheet` for one appearance per legacy helper.
- compare-screenshots: after against [before](../assets/before/README.md),
  judged on tyre, glass and paint crops only. Out of scope: slab shapes.
- screenshot-critique, unprimed, last. Ask specifically whether tyres read
  black and whether anything glows.
- Show the user (preview-shots). Non-blocking: wait about five minutes; if
  silent, decide on the evidence, note it in choices.md, close the shots, go on.

## Delegated

Exact scheme colours within their real references; recipe internals.

## Stays green

Side tint visibly distinguishes the two sides at battle distance; icons
re-derived; every touched GLB validates.
