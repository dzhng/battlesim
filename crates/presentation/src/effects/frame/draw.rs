//! How each running effect draws at the presentation clock: tracers and
//! their tails, rotors, flashes, impacts, sparks, flames, fire's light and
//! blasts, as the effect pass's instances and cast lights. Smoke draws in
//! `smoke.rs`.
use super::*;

pub(super) fn tracer_amplitude(u: f64, retained: f64) -> f64 {
    if retained != 0.0 {
        (retained + (1.0 - retained) * u * u).sqrt()
    } else {
        u
    }
}

/// A round in flight: its glow along the tail, its hot core along the
/// tail's front, and its body and light while this stretch holds the round.
pub(super) fn draw_tracer(
    p: &EffectPresentation,
    e: &Effect,
    style: &TracerStyle,
    age: f64,
    batch: &mut EffectBatch,
) {
    let length = e.length;
    if length <= 1e-6 {
        return;
    }
    let head = length * age / e.span;
    let glow_px = style.line_px.unwrap_or(style.glow.min_px);
    draw_tail(
        e,
        style,
        head,
        e.size,
        &style.glow,
        glow_px,
        style.line_px.is_none(),
        batch,
    );
    if let Some(core) = &style.core {
        draw_tail(
            e,
            style,
            head,
            e.size * core.share,
            core,
            core.min_px,
            false,
            batch,
        );
    }
    // The round is on this stretch from its tick's start to its end (a
    // clock at the tick's end lands a rounding off it).
    if (style.body.is_none() && style.cast.is_none()) || age <= 0.0 || head > length * (1.0 + 1e-6)
    {
        return;
    }
    let a = point_along(e, head.min(length));
    if let Some(body) = &style.body {
        batch.disc(a, body.size_m, p.min_px * 2.0, body.color, 1.0);
    }
    if let Some(c) = &style.cast {
        batch
            .lights
            .offer(a, c.radius_m, c.color, c.intensity, &e.cause);
    }
}

/// The part of `e`'s stretch within `length` behind `head`, in `light`, as
/// streaks fading out toward the tail's end.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_tail(
    e: &Effect,
    style: &TracerStyle,
    head: f64,
    length: f64,
    light: &TracerLight,
    min_px: f64,
    soft: bool,
    batch: &mut EffectBatch,
) {
    let tail = head - length;
    let (path, cum) = (&e.path, &e.cum);
    let retained = style.tail_brightness.unwrap_or(0.0);
    let width = match style.line_px {
        Some(px) if px != 0.0 => -px,
        _ => light.width_m,
    };
    for i in 0..cum.len().saturating_sub(1) {
        let (c0, c1) = (cum[i], cum[i + 1]);
        let (lo, hi) = (c0.max(tail), c1.min(head));
        if hi <= lo || c1 <= c0 {
            continue;
        }
        let (u0, u1) = ((lo - c0) / (c1 - c0), (hi - c0) / (c1 - c0));
        let o = i * 3;
        let a = std::array::from_fn(|k| path[o + k] + (path[o + 3 + k] - path[o + k]) * u0);
        let b = std::array::from_fn(|k| path[o + k] + (path[o + 3 + k] - path[o + k]) * u1);
        let along = [
            tracer_amplitude((lo - tail) / length, retained),
            tracer_amplitude((hi - tail) / length, retained),
        ];
        let boost = style.zoom_boost.unwrap_or(0.0);
        batch.streak(
            a,
            b,
            width,
            min_px,
            light.color,
            light.intensity,
            along,
            soft,
            boost,
        );
    }
}

/// Each rotor as a shutter sees it: its blades smeared over the angle they
/// swept while it was open, a faint disc over its reach while it spins, its
/// hub, and its mast.
pub(super) fn draw_rotors(blur: &RotorBlurStyle, rotors: &[DrawnRotor], batch: &mut EffectBatch) {
    let RotorBlurStyle {
        albedo,
        shutter,
        hub_share,
        min_chord_px,
        disc,
    } = *blur;
    for r in rotors {
        // Past a whole turn every blade covers the whole disc alike.
        let sweep = (2.0 * PI).min(r.turned.abs() * shutter);
        let mut rotor = |radius: f64, sweep: f64, chord: f64, blades: f64, misc: f64| {
            let shape = [ROTOR, chord, blades, misc];
            batch.put([
                v4(r.hub, radius),
                v4(r.axis, sweep),
                v4(r.blade, albedo),
                shape,
            ]);
        };
        rotor(r.radius, sweep, r.chord / r.radius, r.blades, min_chord_px);
        if sweep > 0.0 && disc > 0.0 {
            rotor(r.radius, 0.0, 0.0, 0.0, disc);
        }
        rotor(r.radius * hub_share, 0.0, 0.0, 0.0, 1.0);
        if let Some(mast) = &r.mast {
            let width = 2.0 * r.mast_radius;
            batch.ribbon(mast.top, mast.base, width, 1.0, [albedo; 3], 1.0);
        }
    }
}

/// A burning-out light `age` into its life: 1 − (age/duration)² of its
/// intensity, its reach scaled by `reach`.
pub(super) fn offer_burning_out(
    batch: &mut EffectBatch,
    c: &CastStyle,
    age: f64,
    at: Vec3,
    reach: f64,
    cause: &Arc<str>,
    strength: f64,
) {
    let f = age / c.duration_s;
    let k = c.intensity * strength * (1.0 - f * f);
    batch
        .lights
        .offer(at, c.radius_m * reach, c.color, k, cause);
}

/// A muzzle flash at `at` (the drawn muzzle, else the launch point),
/// pointing where the round went: its glow, its tongue, its fireball and light.
pub(super) fn draw_flash(
    p: &EffectPresentation,
    e: &Effect,
    s: &FlashStyle,
    at: Vec3,
    age: f64,
    batch: &mut EffectBatch,
) {
    if let Some(c) = s.cast.as_ref().filter(|c| age < c.duration_s) {
        offer_burning_out(
            batch,
            c,
            age,
            along(at, e.n, c.forward_m),
            1.0,
            &e.cause,
            1.0,
        );
    }
    let x = age / s.duration_s;
    if x < 1.0 {
        let k = (1.0 - x) * (1.0 - x);
        let size = s.size_m * (0.7 + 0.5 * x);
        batch.glow(
            at,
            size,
            p.min_px,
            s.color,
            s.intensity * k,
            e.rotation,
            0.0,
        );
        let b = along(at, e.n, s.tongue_m * (0.6 + 0.4 * x));
        let width = s.size_m * 0.9;
        batch.streak(
            at,
            b,
            width,
            p.min_px,
            s.color,
            s.intensity * k,
            [1.0, 0.15],
            false,
            0.0,
        );
    }
    if s.fireball_m > 0.0 && age < s.fireball_s {
        let f = age / s.fireball_s;
        let a = along(at, e.n, s.fireball_m * (0.2 + 0.6 * f));
        let radius = s.fireball_m * (0.6 + 0.6 * f.sqrt());
        let heat = s.intensity * 0.25 * (1.0 - f);
        let opacity = (1.0 - f) * (1.0 - f);
        batch.flipbook(
            a,
            radius,
            e.rotation,
            FIRE,
            f * 10.0,
            s.color,
            opacity,
            heat,
            0.5,
            false,
        );
    }
}

/// An impact's puff off the face, rising as it spreads; off a tracer, its
/// light and a hot flash too.
pub(super) fn draw_impact(
    p: &EffectPresentation,
    e: &Effect,
    s: &ImpactStyle,
    tracer: bool,
    age: f64,
    batch: &mut EffectBatch,
) {
    let x = age / e.span;
    let size = e.size * (0.7 + 0.6 * x.sqrt());
    let mut a = along(e.p, e.n, size * 0.45);
    a[2] += e.size * 0.3 * x;
    let opacity = s.opacity * (x * 12.0).min(1.0) * (1.0 - x) * (1.0 - x);
    let frame = (e.frame + x * 0.4) * (DUST_FRAMES - 1.0);
    batch.flipbook(
        a,
        size,
        e.rotation,
        DUST,
        frame,
        s.color,
        opacity,
        0.0,
        size * 0.6,
        false,
    );
    if !tracer {
        return;
    }
    if let Some(c) = s.cast.as_ref().filter(|c| age < c.duration_s) {
        // Off the face, reaching farther for a bigger round.
        let reach = (e.size / s.size_m).sqrt();
        offer_burning_out(batch, c, age, along(e.p, e.n, 0.5), reach, &e.cause, 1.0);
    }
    if s.flash > 0.0 && x < 0.12 {
        let k = 1.0 - x / 0.12;
        // Off the face by its own radius: a sprite in the face would fade into it.
        let r = e.size * 0.35;
        let at = along(e.p, e.n, r);
        batch.glow(
            at,
            r,
            p.min_px,
            p.sparks.color,
            s.flash * k * k,
            e.rotation,
            1.0,
        );
    }
}

/// Sparks flying out under gravity, each a short streak along its path.
pub(super) fn draw_sparks(p: &EffectPresentation, e: &Effect, age: f64, batch: &mut EffectBatch) {
    let s = &p.sparks;
    let fade = 1.0 - age / e.span;
    for v in e.sparks.as_chunks::<3>().0 {
        let speed = crate::world::length(*v);
        let speed = if speed == 0.0 || speed.is_nan() {
            1.0
        } else {
            speed
        };
        let lag = age.min(s.length_m * e.size / speed);
        let (t1, t0) = (age, age - lag);
        let at = |t: f64| -> Vec3 {
            std::array::from_fn(|j| {
                let g = if j == 2 { -0.5 * GRAVITY } else { 0.0 };
                e.p[j] + v[j] * t + g * t * t
            })
        };
        let (width, min_px) = (s.width_m * e.size, p.min_px * 0.5);
        batch.streak(
            at(t0),
            at(t1),
            width,
            min_px,
            s.color,
            s.intensity * fade,
            [0.0, 1.0],
            false,
            0.0,
        );
    }
}

/// A flame: the fire sheet's hot frames, licking up and shrinking away.
pub(super) fn draw_flame(e: &Effect, s: &FlameStyle, age: f64, batch: &mut EffectBatch) {
    let x = age / e.span;
    let mut a = e.p;
    a[2] += s.size_m * e.scale * (0.3 + 1.2 * x);
    let radius = s.size_m * e.scale * (0.8 + 0.4 * libm::sin(PI * x));
    let opacity = (x * 6.0).min(1.0) * (1.0 - x);
    batch.flipbook(
        a,
        radius,
        e.rotation,
        FIRE,
        e.frame + x * 4.0,
        s.tint,
        opacity,
        s.emissive,
        0.3,
        false,
    );
}

/// A burning source's fire light (its glow in the air and its cast light),
/// flickering on the clock, dying down over the burn's last third, and no
/// longer than the flames the publications taken so far made.
pub(super) fn draw_fire_light(
    src: &Source,
    clock: f64,
    noted: f64,
    keep: f64,
    batch: &mut EffectBatch,
) {
    let f = &src.style.flame;
    let age = clock - src.start;
    let burn = src.style.burn_s;
    if age < 0.0 || age >= burn || clock > noted + f.life_s || f.light_intensity <= 0.0 {
        return;
    }
    if f64::from(src.seed) / 4_294_967_296.0 >= keep {
        return; // over the budget
    }
    let phase = f64::from(src.seed % 1000) * 0.01;
    let flicker =
        0.7 + 0.3 * libm::sin(clock * 11.3 + phase) * libm::sin(clock * 5.7 + phase * 1.7);
    let strength = ((burn - age) / burn * 3.0).min(age * 4.0).min(1.0);
    let at = [src.x, src.y, src.top + 0.5];
    batch.glow(
        at,
        f.light_m,
        0.0,
        f.light,
        f.light_intensity * flicker * strength,
        phase,
        0.0,
    );
    if let Some(c) = &f.cast {
        let k = c.intensity * flicker * strength;
        batch.lights.offer(at, c.radius_m, c.color, k, &src.cause);
    }
}

/// A burst: its light, the glow in the air while the fire is hot, the
/// fireball's flipbook over the blast's life, and its first flash.
pub(super) fn draw_blast(p: &EffectPresentation, e: &Effect, age: f64, batch: &mut EffectBatch) {
    let s = &p.blast;
    let x = age / e.span;
    let heat = (1.0 - x * 2.2).max(0.0);
    let c = &s.cast;
    if age < c.duration_s {
        // A burst below the style's lights a smaller pool, and more dimly.
        let share = e.size / s.reference_size_m;
        let at = [e.p[0], e.p[1], e.p[2] + e.size * 0.5];
        offer_burning_out(batch, c, age, at, share.sqrt(), &e.cause, share.min(1.0));
    }
    // The fireball rises through the blast's life; while the fire is hot
    // it sits at the height of the glow round it, as the browser draws it.
    let mut a = e.p;
    a[2] += e.size * (0.55 + 0.5 * x);
    if heat > 0.0 {
        a = e.p;
        a[2] += e.size * 0.5;
        batch.glow(
            a,
            e.size * 1.2,
            0.0,
            s.flash,
            s.spill * heat * heat,
            e.rotation,
            0.0,
        );
    }
    let frame = (x * 1.15).min(1.0) * (FIRE_FRAMES - 1.0);
    let opacity = s.opacity * ((1.0 - x) * 3.0).min(1.0);
    let radius = e.size * (0.75 + 0.5 * x.sqrt());
    let (emissive, soft) = (s.emissive * heat * heat, e.size * 0.5);
    batch.flipbook(
        a, radius, e.rotation, FIRE, frame, s.tint, opacity, emissive, soft, false,
    );
    if age < s.flash_duration_s {
        let k = 1.0 - age / s.flash_duration_s;
        let mut a = e.p;
        a[2] += e.size * 0.4;
        let k = s.flash_intensity * k * k;
        batch.glow(a, e.size * 0.3, p.min_px, s.flash, k, e.rotation, 0.5);
    }
}
