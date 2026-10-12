import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import {
  validateVoiceCatalog,
  RESPONSE_TRIGGERS,
  BARK_TRIGGERS,
  type VoiceCatalog,
  type Trigger,
  type Intensity,
} from "../../src/voices/voiceCatalog";
import voiceJson from "../../../../fixtures/voices.json";
import { duetClient, type ElevenLabsClient } from "./elevenlabs";
import { LOCK, readLock } from "./lock";
import {
  assetsProcessor,
  checkVoices,
  generateBeds,
  generateVoices,
  readProfiles,
  type VoiceSelection,
} from "./voices";

type Unit = { roles?: string[]; roster?: { factions?: string[] } };
const root = resolve(import.meta.dir, "../../../..");
const roster: Record<string, Unit> = {};
for (const name of ["eastern", "europe", "shared", "us"]) {
  const rows = JSON.parse(readFileSync(resolve(root, `fixtures/units/roster/${name}.json`), "utf8"))
    .units as Record<string, Unit>;
  Object.assign(roster, rows);
}
const catalog = validateVoiceCatalog(voiceJson, roster);
const allTriggers = [...RESPONSE_TRIGGERS, ...BARK_TRIGGERS];
const triggersFor = (speaks: string): readonly Trigger[] =>
  speaks === "responses" ? RESPONSE_TRIGGERS : speaks === "barks" ? BARK_TRIGGERS : allTriggers;
function assignment(unit: string, faction: string) {
  const value = catalog.assignments[unit];
  return typeof value === "string" ? value : (value?.[faction] ?? "(missing)");
}
function lineCharacters(
  catalog: VoiceCatalog,
  setId: string,
  trigger: Trigger,
  intensity: "steady" | "stressed",
  take: number,
) {
  const set = catalog.sets[setId];
  return catalog.roles[set.role].lines[trigger][intensity][take].replaceAll(
    "{callsign}",
    set.callsign,
  ).length;
}
function setTotals(setId: string) {
  const set = catalog.sets[setId];
  let clips = 0;
  let characters = 0;
  for (const voice of set.voices) {
    for (const trigger of triggersFor(voice.speaks)) {
      for (const intensity of ["steady", "stressed"] as const) {
        for (let take = 0; take < 2; take++) {
          clips++;
          characters += lineCharacters(catalog, setId, trigger, intensity, take);
        }
      }
    }
  }
  return { clips, characters };
}
function report() {
  let totalClips = 0;
  let totalCharacters = 0;
  for (const setId of Object.keys(catalog.sets).sort()) {
    const totals = setTotals(setId);
    totalClips += totals.clips;
    totalCharacters += totals.characters;
    console.log(
      `SET ${setId}\tvoices=${catalog.sets[setId].voices.length}\tclips=${totals.clips}\tcharacters=${totals.characters}`,
    );
  }
  for (const [unit, row] of Object.entries(roster)) {
    if (!row.roles?.length) continue;
    for (const faction of row.roster?.factions ?? []) {
      const set = assignment(unit, faction);
      const spec = catalog.sets[set];
      console.log(
        `UNIT ${unit}\t${faction}\t${set}\t${spec?.country ?? "?"}\t${spec?.role ?? "?"}\t${spec?.voices.map((voice) => voice.id).join(",") ?? "(missing)"}`,
      );
    }
  }
  console.log(`TOTAL clips=${totalClips} characters=${totalCharacters}`);
  return { clips: totalClips, characters: totalCharacters };
}
function html(path: string) {
  const rows = Object.entries(roster)
    .filter(([, row]) => row.roles?.length)
    .map(([unit, row]) =>
      (row.roster?.factions ?? [])
        .map((faction) => {
          const set = assignment(unit, faction);
          const spec = catalog.sets[set];
          return `<tr><td>${unit}</td><td>${faction}</td><td>${set}</td><td>${spec?.country ?? ""}</td><td>${spec?.role ?? ""}</td><td>${spec?.voices.some((voice) => voice.gender === "female" && (voice.speaks === "responses" || voice.speaks === "all")) ? "yes" : "no"}</td></tr>`;
        })
        .join(""),
    )
    .join("");
  const subtotals = Object.keys(catalog.sets)
    .sort()
    .map((setId) => {
      const totals = setTotals(setId);
      return `<tr><td>${setId}</td><td>${catalog.sets[setId].voices.length}</td><td>${totals.clips}</td><td>${totals.characters}</td></tr>`;
    })
    .join("");
  const scripts = Object.entries(catalog.roles)
    .map(
      ([role, value]) =>
        `<section><h2>${role}</h2><table><tr><th>trigger</th><th>steady</th><th>stressed</th></tr>${allTriggers.map((trigger) => `<tr><td>${trigger}</td><td>${value.lines[trigger].steady.join(" / ")}</td><td>${value.lines[trigger].stressed.join(" / ")}</td></tr>`).join("")}</table></section>`,
    )
    .join("");
  mkdirSync(resolve(root, "specs/battle-voices-music/visualizations"), { recursive: true });
  writeFileSync(
    resolve(root, path),
    `<!doctype html><meta charset="utf-8"><title>Voice sets</title><style>body{font:14px system-ui;background:#111;color:#eee;padding:24px}table{border-collapse:collapse;margin:8px 0 24px;width:100%}td,th{border:1px solid #555;padding:5px;text-align:left;vertical-align:top}section{margin-top:32px}td:nth-child(2),td:nth-child(3){max-width:420px}</style><h1>Battle voice sets</h1><h2>Roster assignments</h2><table><tr><th>unit</th><th>faction</th><th>set</th><th>country</th><th>role</th><th>female response</th></tr>${rows}</table><h2>Per-set generation budget</h2><table><tr><th>set</th><th>voices</th><th>clips</th><th>characters</th></tr>${subtotals}</table>${scripts}`,
  );
}
const USAGE = `usage: bun run generate <command>
  plan [--html path]       the whole library's clips and characters, per set
  beds                     generate the profiles' ambience beds (once)
  voices [selection]       generate, process, transcribe and lock takes
         [--dry-run] [--max-chars N]
  process [selection]      re-render takes whose profile changed; no calls
  check                    prove the lock against the files by hash
  report                   the plan, then what the lock holds
selection: [--set a,b] [--trigger move,attack] [--intensity steady|stressed]
           [--take K] [--only missing|bad|flagged]`;
const [command, ...args] = process.argv.slice(2);
const flag = (name: string) => {
  const at = args.indexOf(`--${name}`);
  return at >= 0 ? args[at + 1] : undefined;
};
const list = (name: string) => flag(name)?.split(",");
function selection(): VoiceSelection {
  const triggers = list("trigger");
  for (const trigger of triggers ?? [])
    if (!allTriggers.includes(trigger as Trigger)) throw new Error(`unknown trigger ${trigger}`);
  const only = flag("only");
  if (only && !["missing", "bad", "flagged"].includes(only)) throw new Error(USAGE);
  return {
    sets: list("set"),
    triggers: triggers as Trigger[] | undefined,
    intensities: list("intensity") as Intensity[] | undefined,
    take: flag("take") === undefined ? undefined : Number(flag("take")),
    only: only as VoiceSelection["only"],
  };
}
const log = (line: string) => console.log(line);
async function voices(client: ElevenLabsClient | null) {
  const result = await generateVoices({
    root,
    catalog,
    client,
    processor: assetsProcessor,
    profiles: readProfiles(),
    select: selection(),
    maxChars: flag("max-chars") === undefined ? undefined : Number(flag("max-chars")),
    dryRun: args.includes("--dry-run"),
    log,
  });
  console.log(
    `TOTAL takes=${result.takes} calls=${result.calls} characters=${result.characters} flagged=${result.flagged} failed=${result.failed}`,
  );
  if (result.stopped) console.log(`STOPPED ${result.stopped.message}`);
  if (result.failed) process.exitCode = 1;
}
function lockReport() {
  const takes = Object.entries(readLock(resolve(root, LOCK)).takes);
  const flagged = takes.filter(([, take]) => take.match.flagged);
  for (const [id, take] of flagged)
    console.log(`FLAGGED ${id}\t"${take.text}" heard "${take.transcript}"`);
  const characters = takes.reduce((sum, [, take]) => sum + take.text.length, 0);
  console.log(
    `LOCK takes=${takes.length} characters=${characters} flagged=${flagged.length} mismatch=${takes.length ? ((100 * flagged.length) / takes.length).toFixed(1) : 0}%`,
  );
}

if (command === "plan") {
  const result = report();
  if (args.includes("--html"))
    html(flag("html") ?? "specs/battle-voices-music/visualizations/voice-sets.html");
  if (!result.clips || !result.characters) process.exitCode = 1;
} else if (command === "beds") {
  const { calls } = await generateBeds({ root, client: duetClient(), log });
  console.log(`TOTAL calls=${calls}`);
} else if (command === "voices") await voices(duetClient());
else if (command === "process") await voices(null);
else if (command === "check") {
  const problems = checkVoices({ root, catalog, profiles: readProfiles() });
  for (const problem of problems) console.log(`FAIL ${problem}`);
  console.log(`check: ${problems.length} problems`);
  if (problems.length) process.exitCode = 1;
} else if (command === "report") {
  report();
  lockReport();
} else {
  console.error(USAGE);
  process.exitCode = 1;
}
