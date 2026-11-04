use super::*;
use crate::settings::{read, validate::range};
use crate::sound::synth;

fn require(ok: bool, path: &str, why: &str) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(format!("sounds.{path}: {why}"))
    }
}
fn id(s: &str) -> bool {
    !s.is_empty()
        && !matches!(s, "__proto__" | "constructor" | "prototype")
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}
fn hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn license(s: &str) -> bool {
    matches!(
        s,
        "CC0-1.0"
            | "Public-Domain-US-Gov"
            | "CC-BY-3.0"
            | "CC-BY-4.0"
            | "CC-BY-NC-3.0"
            | "All-Rights-Reserved"
    )
}
fn source_path(path: &str, prefix: &str) -> bool {
    path.strip_prefix(prefix).is_some_and(|name| {
        !name.is_empty()
            && name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
    })
}
fn date(s: &str) -> bool {
    s.len() == 10
        && s.bytes().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
}
fn level(path: &str, v: f64) -> Result<(), String> {
    range(&format!("sounds.{path}"), v, 0.0, 16.0)
}

impl SoundCatalog {
    pub fn from_value(value: &serde_json::Value) -> Result<Self, String> {
        let catalog: Self = read(value, "sounds")?;
        catalog.check()?;
        Ok(catalog)
    }
    pub fn check(&self) -> Result<(), String> {
        for (name, s) in &self.sources {
            let at = format!("sources.{name}");
            require(id(name) && hash(&s.sha256), &at, "invalid id or sha256")?;
            if s.kind.as_deref() == Some("Generated") {
                require(
                    s.generator.is_some()
                        && s.model.is_some()
                        && s.prompt.is_some()
                        && s.date.as_deref().is_some_and(date)
                        && s.plan.as_deref() == Some("Duet ElevenLabs")
                        && (source_path(&s.path, "assets/source/audio/music/")
                            || source_path(&s.path, "assets/source/audio/effects/")),
                    &at,
                    "invalid generated source",
                )?;
                range(
                    &format!("sounds.{at}.length_s"),
                    s.length_s.unwrap_or(0.0),
                    1.0,
                    600.0,
                )?;
            } else {
                require(
                    s.kind.is_none()
                        && s.label.is_some()
                        && s.author.is_some()
                        && s.license.as_deref().is_some_and(license)
                        && s.url.as_ref().is_some_and(|u| u.starts_with("https://"))
                        && source_path(&s.path, "assets/third-party/audio/")
                        && s.notes.is_some(),
                    &at,
                    "invalid source receipt",
                )?;
            }
        }
        for (name, p) in &self.provenance {
            let at = format!("provenance.{name}");
            let downloaded = p.download.status == "downloaded-unassigned";
            require(
                id(name)
                    && license(&p.license)
                    && p.url.starts_with("https://")
                    && p.source_frames[0] < p.source_frames[1]
                    && !p.processing.is_empty()
                    && (downloaded || p.download.status == "candidate-not-downloaded")
                    && if downloaded {
                        p.sha256.as_deref().is_some_and(hash)
                            && p.path
                                .as_ref()
                                .is_some_and(|p| p.starts_with("assets/third-party/audio/"))
                    } else {
                        p.sha256.is_none() && p.path.is_none()
                    },
                &at,
                "invalid source receipt",
            )?;
        }
        for (name, c) in &self.clips {
            let at = format!("clips.{name}");
            require(
                id(name)
                    && c.frames > 0
                    && c.sample_rate > 0
                    && hash(&c.sha256)
                    && c.url == format!("/audio/clips/{name}.wav"),
                &at,
                "invalid media",
            )?;
            require(
                matches!(
                    c.role.as_str(),
                    "shot"
                        | "launch"
                        | "impact"
                        | "explosion"
                        | "loop"
                        | "reload"
                        | "mechanical"
                        | "other"
                ),
                &format!("{at}.role"),
                "unknown clip role",
            )?;
            match &c.layers {
                Some(layers) => {
                    require(
                        !layers.is_empty()
                            && c.source.is_none()
                            && c.source_rate.is_none()
                            && c.source_frames.is_none()
                            && c.burst.is_none(),
                        &at,
                        "layered clip cannot also be a source crop or burst",
                    )?;
                    for (i, l) in layers.iter().enumerate() {
                        let at = format!("{at}.layers[{i}]");
                        self.crop(&at, &l.source, l.source_rate, l.source_frames)?;
                        range(&format!("sounds.{at}.at_s"), l.at_s, 0.0, 10.0)?;
                        range(&format!("sounds.{at}.gain"), l.gain, 0.0, 4.0)?;
                        range(&format!("sounds.{at}.semitones"), l.semitones, -36.0, 36.0)?;
                        if let Some(hz) = l.lowpass_hz {
                            range(&format!("sounds.{at}.lowpass_hz"), hz, 20.0, 24000.0)?;
                        }
                    }
                }
                None => {
                    self.crop(
                        &at,
                        c.source.as_deref().unwrap_or(""),
                        c.source_rate.unwrap_or(0),
                        c.source_frames.unwrap_or([0, 0]),
                    )?;
                    if let Some(b) = &c.burst {
                        let [lo, hi] = c.source_frames.unwrap();
                        require(
                            c.role == "shot"
                                && !c.looping
                                && b.shots.len() >= 2
                                && b.shots.iter().all(|&[a, z]| lo <= a && a < z && z <= hi),
                            &format!("{at}.burst"),
                            "invalid burst",
                        )?;
                        range(
                            &format!("sounds.{at}.burst.interval_s"),
                            b.interval_s,
                            0.02,
                            1.0,
                        )?;
                    }
                }
            }
        }
        for (name, r) in &self.sounds {
            let at = format!("sounds.{name}");
            require(
                id(name) && (!r.clips.is_empty() || r.synth.is_some()),
                &at,
                "empty recipe or invalid id",
            )?;
            level(&format!("{at}.gain"), r.gain)?;
            range(&format!("sounds.{at}.synth_gain"), r.synth_gain, 0.0, 1.0)?;
            let mut seen = std::collections::HashSet::new();
            let mut cadence = None;
            for clip in &r.clips {
                let c = self
                    .clips
                    .get(clip)
                    .ok_or(format!("sounds.{at}.clips: unknown clip {clip}"))?;
                require(
                    seen.insert(clip) && c.looping == r.looping,
                    &at,
                    "duplicate clip or loop mismatch",
                )?;
                let next = c.cadence();
                require(
                    cadence.is_none_or(|first| first == next),
                    &at,
                    "alternatives play different bursts",
                )?;
                cadence = Some(next);
            }
            if let Some(s) = &r.synth {
                require(
                    synth::loops(s) == Some(r.looping),
                    &format!("{at}.synth"),
                    "unknown synth or loop mismatch",
                )?;
            }
        }
        for (name, t) in &self.music.tracks {
            let at = format!("music.tracks.{name}");
            require(
                id(name) && self.sources.contains_key(&t.source) && hash(&t.sha256),
                &at,
                "invalid track or unknown source",
            )?;
            level(&format!("{at}.gain"), t.gain)?;
        }
        for (mood, tracks) in [
            ("menu", &self.music.moods.menu),
            ("tension", &self.music.moods.tension),
            ("combat", &self.music.moods.combat),
        ] {
            for t in tracks {
                require(
                    self.music.tracks.contains_key(t),
                    &format!("music.moods.{mood}"),
                    "unknown track",
                )?;
            }
        }
        Ok(())
    }
    pub(crate) fn check_shot(
        &self,
        c: &crate::sound::tables::NearFar,
        at: &str,
    ) -> Result<(), String> {
        range(&format!("{at}.gain"), c.gain, 0.0, 16.0)?;
        for (slot, name) in [("near", &c.near), ("far", &c.far)] {
            let r = self.sound(name, &format!("{at}.{slot}"))?;
            require(!r.looping, at, "firing cannot use a loop")?;
            for clip in &r.clips {
                require(
                    matches!(self.clips[clip].role.as_str(), "shot" | "launch"),
                    at,
                    "firing clips must be shots or launches",
                )?;
            }
        }
        require(
            self.cadence(&c.near) == self.cadence(&c.far),
            at,
            "near and far play different bursts",
        )
    }
    fn crop(&self, at: &str, source: &str, rate: u32, [lo, hi]: [u64; 2]) -> Result<(), String> {
        require(
            self.sources.get(source).is_some_and(|s| {
                s.kind.is_none() || source_path(&s.path, "assets/source/audio/effects/")
            }) && rate > 0
                && lo < hi
                && hi <= 9_007_199_254_740_991,
            at,
            "invalid crop or unknown source",
        )
    }
    pub(crate) fn sound(&self, name: &str, at: &str) -> Result<&Recipe, String> {
        self.sounds
            .get(name)
            .ok_or(format!("sounds.{at}: unknown sound {name}"))
    }
    pub(crate) fn cadence(&self, name: &str) -> Cadence {
        self.sounds.get(name).and_then(|r| r.clips.first()).map_or(
            Cadence {
                shots: 1,
                interval_s: 0.0,
            },
            |c| self.clips[c].cadence(),
        )
    }
}
impl LibraryMoods {
    pub fn battle(&self) -> crate::sound::music::Moods {
        crate::sound::music::Moods {
            tension: self.tension.clone(),
            combat: self.combat.clone(),
        }
    }
}
