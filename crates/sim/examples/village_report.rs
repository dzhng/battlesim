//! The village tactical comparison report (encounter.md): comparison scripts
//! over seeds, every (script, seed) trial on one pool sized to the cores,
//! printed in a fixed order as a Markdown table. Each row ends in the trial's
//! final battle digest, so two runs are identical exactly when every digest is.
//!
//!     cargo run -p sim --release --example village_report -- [flags]
//!
//! - `--quick`: the rule-change check — the flank and the 0.75 s ambush, three
//!   seeds, 600 s (long enough for the flank to capture). Without it: every
//!   script, the fixture's repeatability seeds, 900 s (the full report, for
//!   balance tuning and closeout).
//! - `--scripts flank,ambush-0.75`: scripts by id (see `SCRIPTS`).
//! - `--seeds 1,2,3`, `--max-s 600`, `--threads 8`: override the set.
//! - `--save <file>`: write the trials as JSON lines.
//! - `--compare <file | git ref>`: set this run beside a saved one, or beside
//!   the cached run of a ref's sources (e.g. `main`).
//! - `--fresh`: run every trial, ignoring the cache.
//!
//! The cache: when the sources a trial depends on (`CACHE_KEY_PATHS`) are
//! committed and clean, each trial is stored under the main checkout's
//! `throwaway/village-report/`, keyed by those paths' git tree ids. A rerun
//! of the same sources reads it instead of simulating, and `--compare main`
//! reads main's trials from it, so a baseline runs once per main commit that
//! changes the simulation, on whichever checkout ran it first.
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use contract::observation::EncounterResult;
use serde_json::{json, Value};
use sim::village::scripts::Plan;
use sim::village::trial;

/// (id for flags and the cache, report name, fixture variant, plan).
const SCRIPTS: [(&str, &str, &str, Plan); 5] = [
    (
        "push",
        "unsupported-road-push",
        "ordinary",
        Plan::UnsupportedPush,
    ),
    (
        "flank",
        "scout-suppress-flank",
        "ordinary",
        Plan::ScoutSuppressFlank,
    ),
    (
        "ambush-0.75",
        "ordinary-ambush-retreat (0.75 s)",
        "ordinary",
        Plan::AmbushRetreat { delay_s: 0.75 },
    ),
    (
        "ambush-3",
        "ordinary-ambush-retreat (3 s)",
        "ordinary",
        Plan::AmbushRetreat { delay_s: 3.0 },
    ),
    (
        "crossfire",
        "prepared-crossfire (0.75 s)",
        "prepared_crossfire",
        Plan::AmbushRetreat { delay_s: 0.75 },
    ),
];
const QUICK_SCRIPTS: [&str; 2] = ["flank", "ambush-0.75"];
const QUICK_SEEDS: [u64; 3] = [1, 2, 3];
const QUICK_MAX_S: f64 = 600.0;

/// Everything a trial's outcome depends on, relative to the repo root.
const CACHE_KEY_PATHS: [&str; 4] = [
    "crates/sim/src",
    "crates/contract/src",
    "fixtures",
    "crates/sim/examples/village_report.rs",
];

/// Trials by (script id, seed, max_s bits).
type Cache = BTreeMap<(String, u64, u64), Row>;
/// A trial's row, and its run time when it was run rather than read.
type Done = (Row, Option<f64>);

/// One trial's outcome, as the table, the cache and `--save` hold it.
#[derive(Clone, Debug, PartialEq)]
struct Row {
    script: String,
    seed: u64,
    max_s: f64,
    result: String,
    captured_s: Option<f64>,
    blue_cost_lost: f64,
    tanks_lost: u32,
    tanks: u32,
    rejoined: u32,
    rejected: u32,
    digest: String,
}

impl Row {
    fn json(&self) -> String {
        json!({
            "script": self.script, "seed": self.seed, "max_s": self.max_s,
            "result": self.result, "captured_s": self.captured_s,
            "blue_cost_lost": self.blue_cost_lost, "tanks_lost": self.tanks_lost,
            "tanks": self.tanks, "rejoined": self.rejoined,
            "rejected": self.rejected, "digest": self.digest,
        })
        .to_string()
    }

    fn parse(line: &str) -> Option<Row> {
        let v: Value = serde_json::from_str(line).ok()?;
        let u = |k: &str| v[k].as_u64().map(|n| n as u32);
        Some(Row {
            script: v["script"].as_str()?.into(),
            seed: v["seed"].as_u64()?,
            max_s: v["max_s"].as_f64()?,
            result: v["result"].as_str()?.into(),
            captured_s: v["captured_s"].as_f64(),
            blue_cost_lost: v["blue_cost_lost"].as_f64()?,
            tanks_lost: u("tanks_lost")?,
            tanks: u("tanks")?,
            rejoined: u("rejoined")?,
            rejected: u("rejected")?,
            digest: v["digest"].as_str()?.into(),
        })
    }

    fn key(&self) -> (String, u64, u64) {
        (self.script.clone(), self.seed, self.max_s.to_bits())
    }
}

struct Args {
    scripts: Vec<&'static str>,
    seeds: Option<Vec<u64>>,
    max_s: f64,
    threads: usize,
    save: Option<PathBuf>,
    compare: Option<String>,
    fresh: bool,
}

fn args() -> Args {
    let mut a = Args {
        scripts: SCRIPTS.iter().map(|s| s.0).collect(),
        seeds: None,
        max_s: 900.0,
        threads: std::thread::available_parallelism().map_or(4, |n| n.get()),
        save: None,
        compare: None,
        fresh: false,
    };
    let (mut scripts, mut seeds, mut max_s) = (None, None, None);
    let mut it = std::env::args().skip(1);
    let usage = "flags: --quick --scripts a,b --seeds 1,2 --max-s S --threads N --save FILE --compare FILE|REF --fresh";
    while let Some(flag) = it.next() {
        let mut value = || {
            it.next()
                .unwrap_or_else(|| panic!("{flag} needs a value; {usage}"))
        };
        match flag.as_str() {
            "--quick" => {
                scripts.get_or_insert(QUICK_SCRIPTS.join(","));
                seeds.get_or_insert(QUICK_SEEDS.map(|s| s.to_string()).join(","));
                max_s.get_or_insert(QUICK_MAX_S.to_string());
            }
            "--scripts" => scripts = Some(value()),
            "--seeds" => seeds = Some(value()),
            "--max-s" => max_s = Some(value()),
            "--threads" => a.threads = value().parse().expect("--threads takes a count"),
            "--save" => a.save = Some(value().into()),
            "--compare" => a.compare = Some(value()),
            "--fresh" => a.fresh = true,
            _ => panic!("unknown flag {flag}; {usage}"),
        }
    }
    if let Some(list) = scripts {
        a.scripts = list
            .split(',')
            .map(|id| {
                SCRIPTS
                    .iter()
                    .find(|s| s.0 == id)
                    .unwrap_or_else(|| {
                        let ids: Vec<_> = SCRIPTS.iter().map(|s| s.0).collect();
                        panic!("no script {id}; the ids are {}", ids.join(", "))
                    })
                    .0
            })
            .collect();
    }
    a.seeds = seeds.map(|list| {
        list.split(',')
            .map(|s| s.parse().expect("--seeds takes numbers"))
            .collect()
    });
    if let Some(s) = max_s {
        a.max_s = s.parse().expect("--max-s takes seconds");
    }
    a
}

fn main() {
    let a = args();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/village.json");
    let fixture: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let seeds = a
        .seeds
        .clone()
        .unwrap_or_else(|| serde_json::from_value(fixture["repeatability_seeds"].clone()).unwrap());
    let jobs: Vec<(&str, u64)> = a
        .scripts
        .iter()
        .flat_map(|&id| seeds.iter().map(move |&seed| (id, seed)))
        .collect();

    let repo = Repo::find();
    let key = repo.as_ref().and_then(|r| r.key("HEAD", true));
    let cached: Cache = match (&repo, &key) {
        (Some(r), Some(k)) => r.cached(k),
        _ => BTreeMap::new(),
    };
    let reuse = if a.fresh {
        BTreeMap::new()
    } else {
        cached.clone()
    };
    let rows = run(&fixture, &jobs, a.max_s, a.threads, &reuse);
    if let (Some(r), Some(k)) = (&repo, &key) {
        r.store(k, &rows, &cached);
    }
    if key.is_none() {
        eprintln!("(not cached: the simulation sources have uncommitted changes)");
    }

    print_table(&rows);
    if let Some(file) = &a.save {
        let text: String = rows.iter().map(|r| r.json() + "\n").collect();
        std::fs::write(file, text).expect("--save writes its file");
        eprintln!("saved {} trials to {}", rows.len(), file.display());
    }
    if let Some(against) = &a.compare {
        compare(&rows, against, repo.as_ref());
    }
    for r in &rows {
        assert_eq!(
            r.rejected, 0,
            "{} seed {}: a script order was refused",
            r.script, r.seed
        );
    }
}

/// Every job on one pool of `threads` workers, the rows back in job order.
/// Cached trials are read, not run.
fn run(
    fixture: &Value,
    jobs: &[(&str, u64)],
    max_s: f64,
    threads: usize,
    cached: &Cache,
) -> Vec<Row> {
    let started = Instant::now();
    let instructions_before = instructions();
    let next = AtomicUsize::new(0);
    let slots: Mutex<Vec<Option<Done>>> = Mutex::new(vec![None; jobs.len()]);
    std::thread::scope(|s| {
        for _ in 0..threads.clamp(1, jobs.len().max(1)) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(&(id, seed)) = jobs.get(i) else {
                    break;
                };
                let hit = cached.get(&(id.to_string(), seed, max_s.to_bits()));
                let done = match hit {
                    Some(row) => (row.clone(), None),
                    None => {
                        let (_, _, variant, plan) = SCRIPTS.iter().find(|s| s.0 == id).unwrap();
                        let start = Instant::now();
                        let t = trial(fixture, variant, *plan, seed, max_s);
                        let row = Row {
                            script: id.into(),
                            seed,
                            max_s,
                            result: format!("{:?}", t.result),
                            captured_s: t.captured_s,
                            blue_cost_lost: t.blue_cost_lost,
                            tanks_lost: t.tanks_lost,
                            tanks: t.tanks,
                            rejoined: t.rejoined,
                            rejected: t.rejected,
                            digest: format!("{:016x}", t.digest),
                        };
                        (row, Some(start.elapsed().as_secs_f64()))
                    }
                };
                slots.lock().unwrap()[i] = Some(done);
            });
        }
    });
    let done: Vec<Done> = slots.into_inner().unwrap().into_iter().flatten().collect();
    let times: Vec<f64> = done.iter().filter_map(|d| d.1).collect();
    let ran = times.len();
    let spent = instructions().zip(instructions_before).map(|(a, b)| a - b);
    let mut line = format!(
        "{} trials ({ran} run, {} cached) in {:.1} s wall",
        done.len(),
        done.len() - ran,
        started.elapsed().as_secs_f64(),
    );
    if ran > 0 {
        let min = times.iter().copied().fold(f64::INFINITY, f64::min);
        let max = times.iter().copied().fold(0.0, f64::max);
        let mean = times.iter().sum::<f64>() / ran as f64;
        line += &format!(" on {threads} threads; a trial {min:.1}–{max:.1} s (mean {mean:.1})");
        if let Some(n) = spent {
            line += &format!("; {:.0} G instructions retired", n as f64 / 1e9);
        }
        line += &format!("; load {}", load());
    }
    eprintln!("{line}");
    done.into_iter().map(|d| d.0).collect()
}

fn name(id: &str) -> &'static str {
    SCRIPTS.iter().find(|s| s.0 == id).map_or("?", |s| s.1)
}

fn print_table(rows: &[Row]) {
    println!(
        "| script | seed | result | captured s | blue cost lost | tanks lost | rejoined | digest |"
    );
    println!("|---|---|---|---|---|---|---|---|");
    let mut ids: Vec<&str> = Vec::new();
    for r in rows {
        if !ids.contains(&r.script.as_str()) {
            ids.push(&r.script);
        }
    }
    for id in ids {
        let of: Vec<&Row> = rows.iter().filter(|r| r.script == id).collect();
        let name = name(id);
        for r in &of {
            println!(
                "| {name} | {} | {} | {} | {:.0} | {} | {} | `{}` |",
                r.seed,
                r.result,
                r.captured_s.map_or("—".into(), |c| format!("{c:.0}")),
                r.blue_cost_lost,
                r.tanks_lost,
                r.rejoined,
                r.digest
            );
        }
        let s = Summary::of(&of);
        println!(
            "| **{name}** | all | **{}/{} captured** | | **{:.0}** | **{}** (a tank survived in {}) | **{}** | |",
            s.captured, s.trials, s.cost, s.tanks_lost, s.tank_survived, s.rejoined
        );
    }
}

struct Summary {
    trials: usize,
    captured: usize,
    cost: f64,
    tanks_lost: u32,
    tank_survived: usize,
    rejoined: u32,
}

impl Summary {
    fn of(rows: &[&Row]) -> Summary {
        Summary {
            trials: rows.len(),
            captured: rows
                .iter()
                .filter(|r| r.result == format!("{:?}", EncounterResult::Captured))
                .count(),
            cost: rows.iter().map(|r| r.blue_cost_lost).sum(),
            tanks_lost: rows.iter().map(|r| r.tanks_lost).sum(),
            tank_survived: rows.iter().filter(|r| r.tanks_lost < r.tanks).count(),
            rejoined: rows.iter().map(|r| r.rejoined).sum(),
        }
    }
}

/// This run beside a saved one: per script, the totals before → after over
/// the trials both ran, and how many ended in the same digest.
fn compare(rows: &[Row], against: &str, repo: Option<&Repo>) {
    let base: Cache = if std::path::Path::new(against).is_file() {
        std::fs::read_to_string(against)
            .unwrap()
            .lines()
            .filter_map(Row::parse)
            .map(|r| (r.key(), r))
            .collect()
    } else {
        let repo = repo.expect("--compare with a ref needs a git checkout");
        let Some(key) = repo.key(against, false) else {
            panic!("--compare {against}: neither a file nor a git ref");
        };
        repo.cached(&key)
    };
    let missing: Vec<&Row> = rows
        .iter()
        .filter(|r| !base.contains_key(&r.key()))
        .collect();
    if !missing.is_empty() {
        eprintln!(
            "\n{against} has no run of {} of these trials (e.g. {} seed {}). Run the same flags once on a clean checkout of {against} to cache them, or pass a --save file.",
            missing.len(),
            missing[0].script,
            missing[0].seed
        );
    }
    if missing.len() == rows.len() {
        return;
    }
    println!("\nCompared with {against} (before → after):\n");
    println!(
        "| script | trials | same digest | captured | blue cost lost | tanks lost | rejoined |"
    );
    println!("|---|---|---|---|---|---|---|");
    let mut ids: Vec<&str> = Vec::new();
    for r in rows {
        if !ids.contains(&r.script.as_str()) {
            ids.push(&r.script);
        }
    }
    for id in ids {
        let after: Vec<&Row> = rows
            .iter()
            .filter(|r| r.script == id && base.contains_key(&r.key()))
            .collect();
        if after.is_empty() {
            continue;
        }
        let before: Vec<&Row> = after.iter().map(|r| &base[&r.key()]).collect();
        let same = after
            .iter()
            .zip(&before)
            .filter(|(a, b)| a.digest == b.digest)
            .count();
        let (b, a) = (Summary::of(&before), Summary::of(&after));
        let arrow = |x: String, y: String| {
            if x == y {
                x
            } else {
                format!("{x} → **{y}**")
            }
        };
        println!(
            "| {} | {} | {same}/{} | {} | {} | {} | {} |",
            name(id),
            a.trials,
            a.trials,
            arrow(b.captured.to_string(), a.captured.to_string()),
            arrow(format!("{:.0}", b.cost), format!("{:.0}", a.cost)),
            arrow(b.tanks_lost.to_string(), a.tanks_lost.to_string()),
            arrow(b.rejoined.to_string(), a.rejoined.to_string()),
        );
    }
}

/// The git checkout the report runs in, and the shared cache directory.
struct Repo {
    root: PathBuf,
    cache: PathBuf,
}

impl Repo {
    fn find() -> Option<Repo> {
        let root = git(&["rev-parse", "--show-toplevel"])?;
        // The main checkout's throwaway/, so every worktree shares one cache.
        let common = git(&["rev-parse", "--path-format=absolute", "--git-common-dir"])?;
        let main = PathBuf::from(common).parent()?.to_path_buf();
        Some(Repo {
            root: root.into(),
            cache: main.join("throwaway/village-report"),
        })
    }

    /// The cache key of `rev`'s simulation sources: their git object ids.
    /// For the checkout (`worktree`), only when those paths are clean.
    fn key(&self, rev: &str, worktree: bool) -> Option<String> {
        let root = self.root.to_str()?;
        if worktree {
            let mut status = vec!["-C", root, "status", "--porcelain", "--"];
            status.extend(CACHE_KEY_PATHS);
            if !git(&status)?.is_empty() {
                return None;
            }
        }
        let ids: Option<Vec<String>> = CACHE_KEY_PATHS
            .iter()
            .map(|p| git(&["-C", root, "rev-parse", &format!("{rev}:{p}")]))
            .collect();
        Some(format!("{:016x}", sim::digest::of_str(&ids?.join(" "))))
    }

    fn file(&self, key: &str) -> PathBuf {
        self.cache.join(format!("{key}.jsonl"))
    }

    fn cached(&self, key: &str) -> Cache {
        std::fs::read_to_string(self.file(key))
            .unwrap_or_default()
            .lines()
            .filter_map(Row::parse)
            .map(|r| (r.key(), r))
            .collect()
    }

    /// Append the trials this run simulated.
    fn store(&self, key: &str, rows: &[Row], cached: &Cache) {
        use std::io::Write;
        for r in rows {
            if cached.get(&r.key()).is_some_and(|c| c != r) {
                eprintln!(
                    "warning: {} seed {} differs from the cached run of the same sources (a stale binary, or a nondeterministic battle)",
                    r.script, r.seed
                );
            }
        }
        let fresh: String = rows
            .iter()
            .filter(|r| cached.get(&r.key()) != Some(r))
            .map(|r| r.json() + "\n")
            .collect();
        if fresh.is_empty() || std::fs::create_dir_all(&self.cache).is_err() {
            return;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.file(key));
        if let Ok(mut f) = file {
            let _ = f.write_all(fresh.as_bytes());
        }
    }
}

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The machine's one-minute load average, for reading wall times.
fn load() -> String {
    Command::new("sysctl")
        .args(["-n", "vm.loadavg"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

#[path = "common/instructions.rs"]
mod instructions;
use instructions::instructions;
