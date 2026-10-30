//! Mix admission and cross-document references, before voices are prepared.
use super::{catalog::SoundCatalog, tables::AudioMix};
use crate::settings::{
    read,
    validate::{numbers, range},
};
impl AudioMix {
    pub fn from_value(value: &serde_json::Value) -> Result<Self, String> {
        let mix: Self = read(value, "audio")?;
        mix.check()?;
        Ok(mix)
    }
    pub fn check(&self) -> Result<(), String> {
        numbers(self, "audio", &[".threshold_db", ".clip_db"])?;
        self.voices.check()?;
        for (name, has) in [
            ("impacts", self.impacts.contains_key("default")),
            ("vehicles", self.vehicles.contains_key("default")),
            ("fires", self.fires.contains_key("default")),
        ] {
            if !has {
                return Err(format!("audio.{name}: needs a default"));
            }
        }
        let positive = |p: &str, v| range(&format!("audio.{p}"), v, f64::MIN_POSITIVE, f64::MAX);
        for (p, v) in [
            ("cues.shout.lowpass_hz", self.cues.shout.lowpass_hz),
            ("hold_s", self.hold_s),
            ("late_s", self.late_s),
            ("distance.ref_m", self.distance.ref_m),
            ("distance.rolloff", self.distance.rolloff),
            ("air.far_m", self.air.far_m),
            ("air.reverb_s", self.air.reverb_s),
            ("footsteps.stride_m", self.footsteps.stride_m),
            ("footsteps.max_step_m", self.footsteps.max_step_m),
            ("ambience.duck.attack_s", self.ambience.duck.attack_s),
            ("ambience.duck.release_s", self.ambience.duck.release_s),
            ("limiter.attack_s", self.limiter.attack_s),
            ("limiter.release_s", self.limiter.release_s),
            ("music.crossfade_s", self.music.crossfade_s),
        ] {
            positive(p, v)?;
        }
        for (p, v) in [
            ("listener_eye_share", self.listener_eye_share),
            ("distance.floor", self.distance.floor),
            ("air.wet", self.air.wet),
            ("ambience.duck.depth", self.ambience.duck.depth),
            ("music.duck.depth", self.music.duck.depth),
            ("cues.shout.wet", self.cues.shout.wet),
        ] {
            range(&format!("audio.{p}"), v, 0.0, 1.0)?;
        }
        range(
            "audio.distance.max_m",
            self.distance.max_m,
            self.distance.ref_m,
            f64::MAX,
        )?;
        positive("air.far_hz", self.air.far_hz)?;
        range(
            "audio.air.near_hz",
            self.air.near_hz,
            self.air.far_hz,
            24000.0,
        )?;
        range("audio.limiter.ratio", self.limiter.ratio, 1.0, f64::MAX)?;
        range(
            "audio.limiter.clip_db",
            self.limiter.clip_db,
            self.limiter.threshold_db,
            0.0,
        )?;
        if self.budget.voices <= self.budget.loops {
            return Err("audio.budget: voices must exceed loops".into());
        }
        for (kind, v) in &self.vehicles {
            positive(&format!("vehicles.{kind}.idle_rate"), v.idle_rate)?;
            positive(&format!("vehicles.{kind}.load_rate"), v.load_rate)?;
            positive(&format!("vehicles.{kind}.full_speed_mps"), v.full_speed_mps)?;
            positive(
                &format!("vehicles.{kind}.full_traverse_rps"),
                v.full_traverse_rps,
            )?;
        }
        Ok(())
    }
    pub fn check_sounds(&self, c: &SoundCatalog) -> Result<(), String> {
        let sound = |name: &str, at: &str| {
            c.sound(name, at)
                .map(|_| ())
                .map_err(|_| format!("audio.{at}: unknown sound {name}"))
        };
        for (kind, r) in &self.vehicles {
            for (slot, name) in [
                ("engine", Some(r.engine.as_str())),
                ("running", r.running.as_deref()),
                ("turret", r.turret.as_deref()),
                ("reverse", r.reverse.as_deref()),
            ] {
                if let Some(name) = name {
                    sound(name, &format!("vehicles.{kind}.{slot}"))?;
                }
            }
        }
        sound(
            &self.weapon_defaults.blast.near,
            "weapon_defaults.blast.near",
        )?;
        sound(&self.weapon_defaults.blast.far, "weapon_defaults.blast.far")?;
        sound(&self.weapon_defaults.motor, "weapon_defaults.motor")?;
        for hit in self
            .impacts
            .keys()
            .map(String::as_str)
            .chain([super::GLANCE])
        {
            if !self.weapon_defaults.impacts.contains_key(hit) {
                return Err(format!(
                    "audio.weapon_defaults.impacts.{hit}: missing recording"
                ));
            }
        }
        for (hit, name) in &self.weapon_defaults.impacts {
            sound(name, &format!("weapon_defaults.impacts.{hit}"))?;
        }
        for (slot, name) in &self.effects {
            let expected = super::synth::loops(slot)
                .ok_or(format!("audio.effects.{slot}: unknown synth slot"))?;
            if c.sound(name, slot)?.looping != expected {
                return Err(format!("audio.effects.{slot}: effect loop mismatch"));
            }
        }
        for (kind, r) in &self.fires {
            sound(&r.sound, &format!("fires.{kind}.sound"))?;
        }
        for (at, name) in [
            ("ambience.sound", &self.ambience.sound),
            ("footsteps.sound", &self.footsteps.sound),
            ("air_alarm.sound", &self.air_alarm.sound),
            ("missile_warning.sound", &self.missile_warning.sound),
            ("flare_deploy.sound", &self.flare_deploy.sound),
            ("cook_off.blast.near", &self.cook_off.blast.near),
            ("cook_off.blast.far", &self.cook_off.blast.far),
        ] {
            sound(name, at)?;
        }
        for (kind, name) in &self.cues.sounds {
            sound(name, &format!("cues.sounds.{kind}"))?;
        }
        Ok(())
    }
}
