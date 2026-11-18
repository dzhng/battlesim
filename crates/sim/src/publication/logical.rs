//! Complete logical observations before transport encoding.
use super::*;

pub(super) fn pack_record(
    frame: &ObservationFrame,
    looks: &[&str],
    ground: &GroundHeader,
    patch: &FogPatch<'_>,
    runs: impl Iterator<Item = GroundRunPatch>,
    out: &mut Vec<f32>,
    mut ends: Option<&mut Vec<usize>>,
) -> Result<(), String> {
    let fog = &frame.ground_visibility;
    let cells = u64::from(fog.nx) * u64::from(fog.ny);
    let words = cells.div_ceil(32);
    if words > MAX_FOG_WORDS as u64 {
        return Err(format!(
            "fog delivery requires {words} words; admitted bound is {MAX_FOG_WORDS}"
        ));
    }
    let (cells, words) = (cells as usize, words as usize);
    if fog.bits.len() != words {
        return Err("fog field dimensions must match its words".into());
    }
    let fog_floats = if patch.full {
        words * 2
    } else {
        patch.changed.len() * 3
    };
    let [base_lo, base_hi] = limbs(patch.base);
    let [revision_lo, revision_hi] = limbs(patch.revision);
    // Plain ground counters remain exact inside their existing lifetime bound.
    if ground.epoch >= 1 << 24 || ground.revision >= 1 << 24 || ground.base >= 1 << 24 {
        return Err("ground cursor exceeds its exact float range".into());
    }
    let length = packed_len(frame, fog_floats, ground.count)?;
    out.try_reserve_exact(length.saturating_sub(out.len()))
        .map_err(|e| format!("publication allocation: {e}"))?;
    out.clear();
    out.extend([
        frame.tick as f32,
        frame.own.len() as f32,
        frame.identified.len() as f32,
        frame.contacts.len() as f32,
        frame.audible.len() as f32,
        frame.refusals.len() as f32,
        frame.known_props.len() as f32,
        frame.projectiles.len() as f32,
        frame.blasts.len() as f32,
        frame.corpses.len() as f32,
        frame.guided.len() as f32,
        frame.air_transits.len() as f32,
        frame.encounter.map_or(-1.0, |e| e.held_s as f32),
        frame
            .encounter
            .map_or(-1.0, |e| tag(&ENCOUNTER_RESULTS, &e.result)),
        fog.cell_m as f32,
        fog.nx as f32,
        fog.ny as f32,
        fog_floats as f32,
        patch.full as u8 as f32,
        base_lo,
        base_hi,
        revision_lo,
        revision_hi,
        ground.epoch as f32,
        tag(&Side::ALL, &ground.side),
        ground.base as f32,
        ground.revision as f32,
        ground.full as u8 as f32,
        ground.count as f32,
        frame.fallen_bodies.len() as f32,
        frame
            .skirmish
            .as_ref()
            .map_or(-1.0, |s| s.phase as u8 as f32),
        frame
            .skirmish
            .as_ref()
            .map_or(0.0, |s| s.players.len() as f32),
        frame
            .skirmish
            .as_ref()
            .map_or(0.0, |s| s.preparation_remaining_s as f32),
        frame.skirmish.as_ref().map_or(0.0, |s| {
            s.players.iter().map(|p| p.pending.len()).sum::<usize>() as f32
        }),
        frame.skirmish.as_ref().map_or(0.0, |s| {
            s.report.as_ref().map_or(0, |r| r.totals.len()) as f32
        }),
        frame.skirmish.as_ref().map_or(0.0, |s| {
            s.report.as_ref().map_or(0, |r| r.events.len()) as f32
        }),
        frame
            .skirmish
            .as_ref()
            .map_or(0.0, |s| s.objectives.len() as f32),
        frame.skirmish.as_ref().map_or(0.0, |s| s.scores[0] as f32),
        frame.skirmish.as_ref().map_or(0.0, |s| s.scores[1] as f32),
        frame
            .skirmish
            .as_ref()
            .and_then(|s| s.result)
            .map_or(-1.0, |r| match r {
                contract::skirmish::MatchResult::Winner { side } => side.index() as f32,
                contract::skirmish::MatchResult::Draw => 2.0,
            }),
        frame.crashes.len() as f32,
        frame.shouts.len() as f32,
        frame.prop_display_poses.len() as f32,
        frame.grinds.len() as f32,
        frame.smoke_clouds.len() as f32,
    ]);
    for u in &frame.own {
        let (host, footprint, host_seats) = match u.hold.map(|h| h.host) {
            Some(HostRef::Building {
                id,
                center,
                half,
                seats,
            }) => (Some(("building", id)), Some((center, half)), seats as f32),
            Some(HostRef::Carrier { id }) => (Some(("carrier", id)), None, -1.0),
            None => (None, None, -1.0),
        };
        let [host_kind, host_lo, host_hi] = host_cells(host);
        let [host_x, host_y, half_x, half_y] = footprint_cells(footprint);
        let [gx, gy] = u.goal.map_or([f32::NAN; 2], |g| [g[0] as f32, g[1] as f32]);
        out.extend([
            u.id.0 as f32,
            u.owner.map_or(-1.0, |p| f32::from(p.0)),
            u.kind.0 as f32,
            u.position[0] as f32,
            u.position[1] as f32,
            u.position[2] as f32,
            u.yaw as f32,
            gx,
            gy,
            u.policy.map_or(-1.0, |p| tag(&POLICIES, &p)),
            u.direction.map_or(-1.0, |d| tag(&DIRECTIONS, &d)),
            u.reversing as u8 as f32,
            tag(&MOVE_STATES, &u.state),
            u.blocker.map_or(-1.0, |b| b.0 as f32),
            u.route.len() as f32,
            u.steps.len() as f32,
            u.members.len() as f32,
            u.sees.len() as f32,
            tag(&ENGAGEMENTS, &u.engagement),
            u.mounts.len() as f32,
            u.hp as f32,
            tag(&SUPPRESSION_TIERS, &u.suppression),
            u.concealed as u8 as f32,
            u.deployment.map_or(-1.0, |d| d.progress as f32),
            u.deployment.map_or(-1.0, |d| tag(&POSTURES, &d.target)),
            host_kind,
            host_lo,
            host_hi,
            u.hold.map_or(-1.0, |h| tag(&HOLD_PHASES, &h.phase)),
            u.hold.map_or(-1.0, |h| h.progress as f32),
            host_x,
            host_y,
            half_x,
            half_y,
            host_seats,
            u.stock.map_or(-1.0, |n| n as f32),
            tag(&SERVICE_STATUSES, &u.service),
            u.sight.forward as f32,
            u.sight.shape.front as f32,
            u.sight.shape.side as f32,
            u.sight.shape.rear as f32,
            u.sight.range as f32,
            u.sight.eyes.len() as f32,
            u.final_facing as f32,
            u.area.map_or(f32::NAN, |a| a.anchor[0] as f32),
            u.area.map_or(f32::NAN, |a| a.anchor[1] as f32),
            u.area.map_or(f32::NAN, |a| a.radius as f32),
            u.withdrawing as u8 as f32,
            u.fuel_s.map_or(-1.0, |fuel| fuel as f32),
            u.afterburner as u8 as f32,
            u.in_dogfight as u8 as f32,
            u.flares.map_or(-1.0, |f| f.charges as f32),
            u.flares.and_then(|f| f.cooldown).map_or(-1.0, |p| p as f32),
            u.protection.map_or(-1.0, |p| p.charges as f32),
            u.protection
                .and_then(|p| p.cooldown)
                .map_or(-1.0, |p| p as f32),
            u.smoke.map_or(-1.0, |n| n as f32),
            u.smoking as u8 as f32,
            u.shot_at as u8 as f32,
        ]);
    }
    for u in &frame.own {
        out.extend(u.route.iter().flat_map(|p| [p[0] as f32, p[1] as f32]));
        out.extend(u.steps.iter().flat_map(step_row));
        out.extend(
            u.members
                .iter()
                .flat_map(|p| [p[0] as f32, p[1] as f32, p[2] as f32]),
        );
        out.extend(u.member_hp.iter().map(|&hp| hp as f32));
        out.extend(member_ids(
            &u.member_ids,
            &u.member_slots,
            &u.member_active_mounts,
        ));
        let tier = |t: Option<CoverTier>| t.map_or(-1.0, |t| tag(&COVER_TIERS, &t));
        out.extend(u.member_orders.iter().flat_map(|m| {
            [
                m.spot[0] as f32,
                m.spot[1] as f32,
                tier(m.cover_now),
                tier(m.cover_there),
            ]
        }));
        out.extend(u.member_leans.iter().flat_map(lean));
        out.extend(u.sees.iter().map(|id| id.0 as f32));
        out.extend(u.mounts.iter().flat_map(mount_row));
        // One pose per mount: the section is counted by mountCount.
        assert_eq!(u.weapon_poses.len(), u.mounts.len());
        out.extend(u.weapon_poses.iter().flat_map(pose));
        out.extend(
            u.sight
                .eyes
                .iter()
                .flat_map(|p| [p[0] as f32, p[1] as f32, p[2] as f32]),
        );
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for e in &frame.identified {
        let [building_lo, building_hi] = limbs_or_absent(e.building);
        out.extend([
            e.id.0 as f32,
            e.kind.0 as f32,
            e.cost as f32,
            e.position[0] as f32,
            e.position[1] as f32,
            e.position[2] as f32,
            e.yaw as f32,
            e.velocity[0] as f32,
            e.velocity[1] as f32,
            e.members.len() as f32,
            e.weapon_poses.len() as f32,
            e.reversing as u8 as f32,
            e.smoking as u8 as f32,
            building_lo,
            building_hi,
        ]);
    }
    for e in &frame.identified {
        out.extend(
            e.members
                .iter()
                .flat_map(|p| [p[0] as f32, p[1] as f32, p[2] as f32]),
        );
        out.extend(member_ids(
            &e.member_ids,
            &e.member_slots,
            &e.member_active_mounts,
        ));
        out.extend(e.member_leans.iter().flat_map(lean));
        out.extend(e.weapon_poses.iter().flat_map(pose));
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for c in &frame.contacts {
        out.extend([
            c.id.0 as f32,
            tag(&CONTACT_SOURCES, &c.source),
            c.center[0] as f32,
            c.center[1] as f32,
            c.center[2] as f32,
            tag(&LAYERS, &c.layer),
            if c.aloft { 1.0 } else { 0.0 },
            c.radius as f32,
            c.evidence_tick as f32,
            c.expires_tick as f32,
            c.kind.map_or(-1.0, |k| k.0 as f32),
        ]);
        out.extend(
            (0..HEARD_WORDS).map(|w| {
                ((c.heard >> (w * HEARD_WORD_BITS)) & ((1 << HEARD_WORD_BITS) - 1)) as f32
            }),
        );
        out.extend([
            c.bearing_deg.unwrap_or(-1.0) as f32,
            c.range_m.unwrap_or(-1.0) as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for a in &frame.audible {
        out.extend([
            a.listener.0 as f32,
            tag(&SOUND_CATEGORIES, &a.category),
            a.sector as f32,
            tag(&SOUND_BANDS, &a.band),
            a.moving as u8 as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for refusal in &frame.refusals {
        let [lo, hi] = limbs(refusal.unit.0);
        out.extend([
            lo,
            hi,
            ORDER_ERROR_NAMES
                .iter()
                .position(|name| *name == refusal.reason.reason_name())
                .expect("every order error has a publication name") as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for p in &frame.projectiles {
        let [lo, hi] = limbs_or_absent(p.shooter_member);
        let n = p.impact_normal.unwrap_or([0.0; 3]);
        out.extend([
            p.path.len() as f32,
            p.ricochets.len() as f32,
            p.own as u8 as f32,
            p.kind as f32,
            lo,
            hi,
            tag(&SEGMENT_HITS, &p.hit),
            n[0] as f32,
            n[1] as f32,
            n[2] as f32,
        ]);
    }
    for p in &frame.projectiles {
        out.extend(
            p.path
                .iter()
                .flat_map(|q| [q[0] as f32, q[1] as f32, q[2] as f32]),
        );
        out.extend(p.ricochets.iter().flat_map(|r| {
            [
                r.point as f32,
                r.normal[0] as f32,
                r.normal[1] as f32,
                r.normal[2] as f32,
            ]
        }));
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for b in &frame.blasts {
        out.extend([
            b.point[0] as f32,
            b.point[1] as f32,
            b.point[2] as f32,
            b.radius as f32,
            b.kind as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for g in &frame.guided {
        let [lo, hi] = limbs(u32::try_from(g.id).expect("a guided id fits two limbs"));
        out.extend([
            lo,
            hi,
            g.position[0] as f32,
            g.position[1] as f32,
            g.position[2] as f32,
            g.point[0] as f32,
            g.point[1] as f32,
            g.point[2] as f32,
            g.supported as u8 as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for t in &frame.air_transits {
        out.extend([
            t.kind.0 as f32,
            t.point[0] as f32,
            t.point[1] as f32,
            t.point[2] as f32,
            t.heading as f32,
            t.speed_mps as f32,
            t.edge_tick as f32,
            tag(&AIR_TRANSIT_DIRECTIONS, &t.dir),
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for c in &frame.corpses {
        let [lo, hi] = limbs(c.soldier);
        out.extend([
            c.position[0] as f32,
            c.position[1] as f32,
            c.position[2] as f32,
            c.own as u8 as f32,
            lo,
            hi,
            c.kind.0 as f32,
            c.slot as f32,
            c.yaw as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for p in &frame.known_props {
        let [replaces_lo, replaces_hi] = limbs_or_absent(p.replaces);
        let [id_lo, id_hi] = limbs(p.id);
        let [building_lo, building_hi] = limbs_or_absent(p.building);
        let [owner_lo, owner_hi] = limbs_or_absent(p.structure_owner);
        let [authored_lo, authored_hi] = limbs_or_absent(p.authored_prop);
        out.extend([
            p.kind.0 as f32,
            p.center[0] as f32,
            p.center[1] as f32,
            p.yaw as f32,
            p.half_extents[0] as f32,
            p.half_extents[1] as f32,
            p.half_extents[2] as f32,
            p.base_z as f32,
            replaces_lo,
            replaces_hi,
            p.destroyed as u8 as f32,
            id_lo,
            id_hi,
            building_lo,
            building_hi,
            owner_lo,
            owner_hi,
            authored_lo,
            authored_hi,
            p.wreck_of.map_or(-1.0, |t| t.0 as f32),
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for f in &frame.fallen_bodies {
        let [lo, hi] = limbs(f.prop);
        out.extend([
            lo,
            hi,
            f.at[0] as f32,
            f.at[1] as f32,
            f.toward[0] as f32,
            f.toward[1] as f32,
            f.tick as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    if let Some(skirmish) = &frame.skirmish {
        for player in &skirmish.players {
            for p in &player.pending {
                out.extend(pending_row(player.index, p)?);
            }
        }
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    if let Some(report) = frame.skirmish.as_ref().and_then(|s| s.report.as_ref()) {
        for totals in &report.totals {
            out.extend([
                totals.spawned as f32,
                totals.spent as f32,
                totals.killed as f32,
                totals.refunded as f32,
            ]);
        }
        if let Some(ends) = ends.as_mut() {
            ends.push(out.len());
        }
        for event in &report.events {
            out.extend(report_event_row(event)?);
        }
    } else if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    if let Some(skirmish) = &frame.skirmish {
        for (i, o) in skirmish.objectives.iter().enumerate() {
            out.extend([
                i as f32,
                o.center[0] as f32,
                o.center[1] as f32,
                o.radius_m as f32,
                o.owner.map_or(-1.0, |s| s.index() as f32),
                o.capturing.map_or(-1.0, |s| s.index() as f32),
                o.capture_progress as f32,
                o.contested as u8 as f32,
            ]);
        }
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for c in &frame.crashes {
        out.extend(crash_row(c));
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for s in &frame.shouts {
        let (own, id) = match s.speaker {
            ShoutSpeaker::Own(id) => (1.0, id.0),
            ShoutSpeaker::Seen(id) => (0.0, id.0),
        };
        let [lo, hi] = limbs(id);
        out.extend([
            own,
            lo,
            hi,
            s.kind.0 as f32,
            s.position[0] as f32,
            s.position[1] as f32,
            s.position[2] as f32,
            tag(&ShoutTrigger::ALL, &s.trigger),
            tag(&INTENSITIES, &s.intensity),
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    if let Some(skirmish) = &frame.skirmish {
        for player in &skirmish.players {
            out.extend(player_row(player));
        }
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for p in &frame.prop_display_poses {
        out.extend([
            p.prop_id as f32,
            p.center[0] as f32,
            p.center[1] as f32,
            p.yaw as f32,
            p.base_z as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for g in &frame.grinds {
        out.extend([
            g.point[0] as f32,
            g.point[1] as f32,
            g.point[2] as f32,
            g.heading[0] as f32,
            g.heading[1] as f32,
        ]);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    for c in &frame.smoke_clouds {
        out.extend(smoke_cloud_row(c, looks)?);
    }
    if let Some(ends) = ends.as_mut() {
        ends.push(out.len());
    }
    let word = |i: usize| {
        let value = fog.bits[i];
        if i + 1 == words && !cells.is_multiple_of(32) {
            value & ((1u32 << (cells % 32)) - 1)
        } else {
            value
        }
    };
    if patch.full {
        for i in 0..words {
            out.extend(limbs(word(i)));
        }
    } else {
        for &i in patch.changed {
            // MAX_FOG_WORDS keeps the word index exact in a single float.
            out.push(i as f32);
            out.extend(limbs(word(i)));
        }
    }
    let mut encoded = 0;
    for run in runs {
        if encoded >= ground.count
            || run.tile >= 1 << 24
            || run.len == 0
            || usize::from(run.start) + usize::from(run.len) > 256
        {
            out.clear();
            return Err("ground run exceeds its exact tile-local encoding".into());
        }
        let c = run.marks;
        out.extend([
            run.tile as f32,
            u32::from(run.start) as f32 + u32::from(run.len) as f32 * 256.0,
            (u32::from(c.crater) + (u32::from(c.scorch) << 8)) as f32,
            (u32::from(c.tracks) + (u32::from(c.trampled) << 8) + (u32::from(c.cleared) << 16))
                as f32,
        ]);
        encoded += 1;
    }
    if encoded != ground.count || out.len() != length {
        out.clear();
        return Err("publication values do not match their counts".into());
    }
    Ok(())
}

fn packed_len(frame: &ObservationFrame, fog: usize, runs: usize) -> Result<usize, String> {
    let mut length = HEADER.len();
    let mut add = |rows: usize, width: usize| -> Result<(), String> {
        length = rows
            .checked_mul(width)
            .and_then(|n| length.checked_add(n))
            .filter(|n| *n <= MAX_PUBLICATION_BYTES / 4)
            .ok_or("publication exceeds its 64 MiB atomic allocation allowance")?;
        Ok(())
    };
    add(
        frame.skirmish.as_ref().map_or(0, |s| {
            s.players.iter().map(|p| p.pending.len()).sum::<usize>()
        }),
        PENDING_FIELDS.len(),
    )?;
    add(
        frame.skirmish.as_ref().map_or(0, |s| s.players.len()),
        PLAYER_FIELDS.len(),
    )?;
    add(
        frame
            .skirmish
            .as_ref()
            .and_then(|s| s.report.as_ref())
            .map_or(0, |r| r.totals.len()),
        REPORT_TOTAL_FIELDS.len(),
    )?;
    add(
        frame
            .skirmish
            .as_ref()
            .and_then(|s| s.report.as_ref())
            .map_or(0, |r| r.events.len()),
        REPORT_EVENT_FIELDS.len(),
    )?;
    add(
        frame.skirmish.as_ref().map_or(0, |s| s.objectives.len()),
        OBJECTIVE_FIELDS.len(),
    )?;
    add(frame.own.len(), OWN_FIELDS.len())?;
    for u in &frame.own {
        add(u.route.len(), 2)?;
        add(u.steps.len(), STEP_FIELDS.len())?;
        add(u.members.len(), 3)?;
        add(u.member_hp.len(), 1)?;
        add(
            u.member_ids
                .len()
                .min(u.member_slots.len())
                .min(u.member_active_mounts.len()),
            4,
        )?;
        add(u.member_orders.len(), 4)?;
        add(u.member_leans.len(), LEAN_FIELDS.len())?;
        add(u.sees.len(), 1)?;
        add(u.mounts.len(), MOUNT_FIELDS.len())?;
        add(u.weapon_poses.len(), POSE_FIELDS.len())?;
        add(u.sight.eyes.len(), 3)?;
    }
    add(frame.identified.len(), IDENTIFIED_FIELDS.len())?;
    for u in &frame.identified {
        add(u.members.len(), 3)?;
        add(
            u.member_ids
                .len()
                .min(u.member_slots.len())
                .min(u.member_active_mounts.len()),
            4,
        )?;
        add(u.member_leans.len(), LEAN_FIELDS.len())?;
        add(u.weapon_poses.len(), POSE_FIELDS.len())?;
    }
    add(frame.contacts.len(), CONTACT_FIELDS.len())?;
    add(frame.audible.len(), AUDIBLE_FIELDS.len())?;
    add(frame.refusals.len(), REFUSAL_FIELDS.len())?;
    add(frame.projectiles.len(), PROJECTILE_FIELDS.len())?;
    for p in &frame.projectiles {
        add(p.path.len(), 3)?;
        add(p.ricochets.len(), 4)?;
    }
    add(frame.blasts.len(), BLAST_FIELDS.len())?;
    add(frame.guided.len(), GUIDED_FIELDS.len())?;
    add(frame.air_transits.len(), AIR_TRANSIT_FIELDS.len())?;
    add(frame.corpses.len(), CORPSE_FIELDS.len())?;
    add(frame.known_props.len(), KNOWN_PROP_FIELDS.len())?;
    add(frame.fallen_bodies.len(), FALLEN_BODY_FIELDS.len())?;
    add(frame.crashes.len(), CRASH_FIELDS.len())?;
    add(frame.shouts.len(), SHOUT_FIELDS.len())?;
    add(
        frame.prop_display_poses.len(),
        PROP_DISPLAY_POSE_FIELDS.len(),
    )?;
    add(frame.grinds.len(), GRIND_FIELDS.len())?;
    add(frame.smoke_clouds.len(), SMOKE_CLOUD_FIELDS.len())?;
    add(fog, 1)?;
    add(runs, GROUND_FIELDS.len())?;
    Ok(length)
}
