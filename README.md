# battlegame

**A browser real-time tactics game where reconnaissance, physical fire, and
positioning decide the battle.**

Every round is a flown projectile. Every side sees only what its own units can
see. Fresh seeded battlefields put the same roster into a different fight each
time, while the simulation keeps every replay exact.

## The fight

You command squads, vehicles, and aircraft across generated maps. Scout before
you expose yourself, use bodies and buildings as cover, break the line that
holds an enemy in place, and make every shot count. The game aims for the
realism of a war film: physical enough to feel legible, dramatic enough to be
fun.

The rules stay small because they come from what units and weapons physically
have. A weapon's target classes decide what it can engage. A unit's hull,
sensors, and movement properties decide where it can go and what it can know.
There are no special-case “tank trap” or “air-defence unit” rules hiding behind
the content.

## What makes it different

- **One battle, one authority.** The deterministic simulation owns commands,
  visibility, combat, and replay digests. The browser and native client observe
  it.
- **Information is part of the fight.** You do not receive the enemy's hidden
  state; every side gets the evidence its own units earned.
- **Terrain has consequences.** Bodies block movement and rounds according to
  their authored physical properties. Cover, destruction, and line of fire are
  visible outcomes of those properties.
- **The battlefield stays reproducible.** Seeds, commands, and digests make a
  battle you can replay, compare, and investigate.

## Try it

You need Bun, Rustup, `wasm-pack`, and a WebGPU browser.

```bash
bun run setup
bun run dev
```

`bun run setup` installs dependencies, installs the repository's pre-commit
checks, prepares Playwright Chromium, and checks the audio-test prerequisites.
On Windows, [`tools/setup-windows.ps1`](tools/setup-windows.ps1) checks the
required tools and prepares the browser or [native](native/client/README.md)
client.

A battle's address is shareable: it names the admitted map and the player's
choices, so a seeded battle opens the same fight again. The developer menu
holds the battle lab, tutorial, and development tools.

## Find your way around

[development.md](development.md) owns the engineering model: which subsystem
owns each concept (with a link to each subsystem's README), the game rules, the
check ladder, and the contributor workflow. The [specs](specs/README.md) hold
active plans and the durable rationale for shipped features. Agents should start
with [AGENTS.md](AGENTS.md).
