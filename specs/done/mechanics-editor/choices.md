# Mechanics editor choices

These are implementation choices left open by the agreed contracts. All are
sound; the first two deserve the closest review because they add conventions
future authoring tools must preserve.

## Sound — medium confidence

### Recover a save through a scratch journal

**When:** admission/publication integration (`542b0cec`, `117c45a4`).
**Gap:** the plan required recovery but did not choose a filesystem protocol.
When a save replaces several files, a small record in ignored scratch storage
holds their original and proposed text plus the writer's process id. Another
reader refuses to interfere with a live writer. After a crashed writer, it
restores original text before serving readers. If a detected outside edit changed a file in
between, recovery refuses to overwrite that person's edit. Ordinary files do
not offer a single atomic replacement of a whole group, so this convention
provides recovery rather than pretending the group is an operating-system
transaction. **Reach:** every multi-file editor save and future recovery tool.
**Verdict:** sound; exact text comparisons preserve detected outside edits and exclusive
journal creation serializes writers.

### Name local soldier variants after the selected unit

**When:** publication integration (`542b0cec`). **Gap:** the plan chose local
soldier overrides but left catalog naming open. Editing one squad's soldier
health creates a soldier definition named from that squad and the original
soldier kind, then changes that squad's slots to use it. An unrelated squad keeps
using the original soldier; descendants that inherit the selected row’s slots
follow normal inheritance. Restoring the last override removes the local row;
redundant copied slots are removed only when the Rust resolver proves the
result equivalent. **Reach:** the authored catalog gains a naming convention,
without a new schema or an editor-only database. **Verdict:** sound; the
existing inheritance system remains the source of truth.

### Discard edits that a structural replacement supersedes

**When:** structural draft cleanup (`286320a9`). **Gap:** retaining edits across
filtering was agreed, but the plan did not choose what to do when a parent
control removes its children. Replacing or restoring a parent discards that
subtree's edits, including unfinished text. Removing a soldier kind's final slot
drops that kind's edits scoped to the selected unit. For example, removing a
mount after entering an invalid shot interval removes both the mount and its
now-unreachable interval error; unrelated edits survive. **Reach:** structural
changes deliberately supersede child input, while search and selection preserve it.
**Verdict:** sound; removed fields cannot leave hidden errors blocking Save.

## Sound — high confidence

### Build native admission once per development startup

**When:** native admission and integration (`541249d7`, `542b0cec`). **Gap:**
the plan required Rust admission without selecting its host interface.
Development builds a small native executable; each preview passes current JSON
text to it over standard input. It uses the same admission rules as battle
startup. Recompiling for each request would waste time because Rust embeds
shipped fixture files too. After editing Rust validation code, rebuild the
executable. **Reach:** the existing development command gains one build step,
with no extra server or dependency. **Verdict:** sound; simulation validation
retains one owner and fixture edits stay fast.

### Keep Rust's exact generated JSON text

**When:** canonical publication (`117c45a4`). **Gap:** the plan required catalog
synchronization but did not specify who formats its bytes. Rust returns the
resolved catalog as formatted JSON. JavaScript parses it for display while
saving the original returned text. Reformatting it in JavaScript would change
numbers such as `1.0` to `1`, failing the existing exact freshness check despite
equal values. Unchanged authored documents also retain their original text.
**Reach:** new consumers must preserve native output when publishing the
catalog. **Verdict:** sound; the generator owns both values and representation.

### Capture rules before loading gameplay consumers

**When:** lifecycle integration (`a837bd40`), revised by the user's HUD scope
decision on 2026-10-04. Development fetches accepted game and catalog data before
loading modules that derive presentation values. Battle starts, restarts and
diagnostic resets keep that page's captured generation. Saved edits take effect
after a manual page reload; neither Save nor Restart refreshes the page.
**Why:** explicit developer refresh avoids settings-lifetime machinery while
workers and their presentation observe the same generation.

### Isolate Vite's cache by checkout

**When:** integration (`542b0cec`). **Gap:** shared dependencies could also
share optimized modules between worktrees. Each checkout stores Vite's generated
cache in its own ignored scratch folder while sharing installed dependencies.
Opening an editor in one checkout therefore cannot reuse another checkout's
source modules. **Reach:** development cache storage changes; dependencies do
not. **Verdict:** sound; generated output belongs to the source that produced it.

### Confirm discarding edits inside the editor

**When:** editor integration (`98b3b2a9`). **Gap:** stale-source recovery did not
specify a confirmation surface. Clicking Reload sources with a draft first arms
an explicit Discard draft and reload action. The second click discards edits and
fetches current files. This keeps accidental data loss avoidable without a
browser modal. **Reach:** outside-edit recovery has a two-step interaction.
**Verdict:** sound; the action is visible and reversible before confirmation.

### Keep a validated JSON control for structural mount lists

**When:** editor forms (`98b3b2a9`). **Gap:** the plan required complete gameplay
editing without choosing a control for adding or removing whole named mounts.
Ordinary values inside each mount have explained form controls. Replacing the
entire mount list uses an explicit JSON text box, which retains invalid text and
blocks Preview while the text does not parse. Preview then submits the parsed
list to native admission before showing accepted replacements. This supports
structural changes without inventing a second mount schema. **Reach:** adding a
mount is more technical than adjusting an existing one; the raw control remains
an intentional escape hatch alongside structured fields. **Verdict:** sound;
existing rows stay easy to tune and structural edits remain checked.

### Correct routes from actual boundary points

**When:** closeout prerequisite (`130b3214`). **Gap:** the editor plan did not
anticipate a movement failure exposed by the earlier projectile tuning. A soldier
stood clear of a wall, but the centre of his containing grid cell lay inside it.
The route search rejected a valid escape around another soldier, leaving him
walking into occupied room. It now checks the actual segments, including exact
start and end points, using the same body-clearance owner as route simplification.
Ground corner restrictions and existing physical clearances remain. **Reach:**
infantry’s fine local routes use this correction; battle outcomes may change because units
can move where the existing physical rules already permit. **Verdict:** sound;
it restores the movement contract instead of weakening its stall bound or
retuning weapons.

### Retain the current cover calibration owner

**When:** closeout test repair (`e907ecf8`). **Gap:** the full gate still contained
an old assertion that every covered squad must lose a fixed percentage less
health. The accepted cover-balance rationale already supersedes that claim:
formation can turn a miss into a hit on a neighbour, and obstacles also intercept
rounds. The obsolete test is removed; the existing default Cargo report tests
retain controlled single-soldier eligibility, equal incoming fire, reduced hits,
unchanged damage per hit and ordered tier strength. **Reach:** gameplay tuning is
judged by the current calibration contract, without raising cover coefficients
to satisfy a stale test. **Verdict:** sound; one owner keeps the stronger,
accepted proof rather than maintaining contradictory tests.

### Read preview values from accepted soldier identities

**When:** final preview integration (`f77853f8`). **Gap:** local soldier edits
create or remove definitions, so a preview needs to know which accepted row
represents the selected soldier after those operations. The publisher returns
that identity alongside the admitted catalog. The form reads its after value
from that catalog instead of guessing the clone name or projecting a default.
For example, restoring the last local health override can remove the clone;
preview then reads the original soldier's accepted health. **Reach:** consumers
of preview must use its returned identity rather than reconstructing publication.
**Verdict:** sound; the code that creates and removes rows also owns their identity.
