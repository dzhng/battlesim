# 03 — Spike: rig, clips, tank articulation

**Status:** planned. **Depends on:** 00 (and the user accepting the licences). **Lane:** spike (throwaway).

## Contract

Can a CC0 humanoid rig plus our own Blender-scripted modern kit and clips give acceptable soldiers? Can a Blender-scripted tank carry named turret, gun and HMG nodes? All headless (`/Applications/Blender.app/Contents/MacOS/Blender -b --python`), with no Blender MCP server.

## API seam

A spike worktree builds:
- one rifleman on the Quaternius rig, with a helmet, vest and rifle;
- the clip roles: idle, walk, run, kneel_fire, prone_pinned, death;
- the Universal Animation Library clips it has, with the rest authored in Blender;
- one tank with `turret`, `gun`, `muzzle`, `hmg`, road wheels and `track_L/R`.

Rendered as Blender contact sheets (8 views) and animation strips (8 phases per clip).

## What you can run or see

`spikes/03.md`, with the sheets and strips, and the clip coverage table (library vs authored).

## Verification

**Verify:**
- the licence is accepted;
- basis conversion to Z-up with +X forward;
- the bind hierarchy;
- which clips the library has and which were authored.

**Kill criteria**, any of:
- no accepted licence;
- the rig can't hold a two-handed rifle pose acceptably;
- an unprimed critique of the sheets against `defilade/x-saving-wounded-f4.jpg` (two soldiers by the wall) and `warno/steam-warno-1.jpg` (nearest tank) calls them placeholder or toy.

**Fallback:** another CC0 rig with our own clips. If the look still fails, a deliberately stylized soldier, taken to the user and never chosen silently.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Decision budget

- **Delegated:** Candidate choice and the method for authoring missing clips.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

Slices 11, 21 and 22 consume the verdict.
