//! Build one loaded battle frame from the sampled publication.
use super::*;
use crate::view::Carries;

impl Loaded {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn frame(
        &mut self,
        sample: &FrameSample,
        view: &View,
        ui: &Ui,
        world: &World,
        ground: Option<&Ground>,
        unit_scale: f64,
        f: &mut Frame,
        models: &mut Vec<Model>,
        corpse_palettes: &mut posing::CorpsePalettes,
    ) {
        let Loaded {
            input,
            units,
            tables,
            inverse_binds,
            feed,
            driver,
            drawn,
            effects,
            batch,
            lights,
            cook_offs,
            transits,
            pieces,
            shadows,
            jets,
            indoor,
            allied,
            shadow_fall,
            scratch,
            ..
        } = self;
        let time = sample.time;
        let poses = driver.update(feed.frame(sample, units, &input.effects), units, ground);
        drawn.update(poses);
        let rotors = drawn.rotors();
        let side: &str = &input.side;
        let height = |x: f64, y: f64| ground.map_or(0.0, |g| g.source.grid.height(x, y));
        let transit_poses = transits.tracks(time * input.tick_hz, input.tick_hz, &units.transit);
        aircraft::high_air_shadows(effects, poses, &transit_poses, units, side, ground, shadows);
        aircraft::observe_jets(effects, drawn, time, jets);
        let wind = ground.map(|g| &g.biome.wind);
        effects.build(time, batch, Some(drawn), &rotors, shadows, wind);
        let counts = effects.counts(time, batch);
        f.light_wrap = batch.lights.wrap as f32;
        lights.pack(&batch.lights, view);
        f.counts.running = counts.running.map(|n| n as u32);
        f.counts.lights = counts.lights as u32;
        f.counts.dropped = batch.dropped as u32;

        let detail = units.detail.scaled(unit_scale);
        let xray = input.xray;
        let o = &sample.observation;
        indoor.clear();
        indoor.extend(
            o.own
                .iter()
                .filter(|u| u.hold.as_ref().is_some_and(|h| &*h.phase != "entering"))
                .map(|u| u.id),
        );
        allied.clear();
        if let Some(player) = ui.player {
            allied.extend(
                o.own
                    .iter()
                    .filter(|u| u.owner.is_some_and(|owner| owner != f64::from(player)))
                    .map(|u| u.id),
            );
        }
        let colour_of = |unit_side: &str, unit: f64| -> [f64; 4] {
            if !ui.xray || unit_side != side || indoor.contains(&unit) {
                [0.0; 4]
            } else if allied.contains(&unit) {
                xray.ally
            } else if ui.selected.contains(&(unit as u32)) {
                xray.selected
            } else {
                xray.own
            }
        };
        let body = crate::world::BodyBox::new(units, input.soldier);
        let art = UnitArt {
            units,
            tables,
            inverse_binds,
            world,
            view,
            detail,
            stand_in: input
                .stand_in
                .as_ref()
                .map(|appearance| super::posing::UnitStandIn {
                    appearance,
                    body: &body,
                }),
        };
        // Soldiers off screen, as the gates count bodies drawn, carded or culled.
        let culled = art.soldiers(&poses.soldiers, colour_of, &mut scratch.worlds, f, models);
        // Where an aircraft's shadow lies, and the pass that draws it: a
        // low-air airframe's shadow lands its height along the sun's fall; a
        // high-air one, drawn in its own pass, casts none, and its shadow
        // lies on the ground beneath it.
        let aircraft_shadow_at = |kind: &str, p: Vec3| {
            let high = units.types.get(kind).is_some_and(|t| t.high_air());
            let ground_z = height(p[0], p[1]);
            if high {
                ([p[0], p[1], ground_z], HIGH_PASS)
            } else {
                (aircraft_shadow(p, ground_z, *shadow_fall), VIEW_PASS)
            }
        };
        // Vehicles and aircraft; an aircraft draws while it or its shadow
        // is in view.
        art.vehicles(
            &poses.vehicles,
            |v| {
                if v.airborne {
                    let (at, pass) = aircraft_shadow_at(&v.kind, v.position);
                    (Some(at), pass)
                } else {
                    (None, VIEW_PASS)
                }
            },
            colour_of,
            scratch,
            f,
            models,
        );
        // Edge transits use the same appearance, articulation and shadow as a
        // live aircraft, but have no unit identity for selection or cards.
        for t in &transit_poses {
            let Some(name) = units.resolve(&t.kind, side, 0.0, 0, None, None) else {
                continue;
            };
            f.counts.aircraft += 1;
            let far = bounds(units, name, 0);
            let (shadow, pass) = aircraft_shadow_at(&t.kind, t.position);
            let tier = model_detail(
                &detail,
                view,
                t.position,
                far.size,
                far.radius,
                Carries::Nothing,
                Some(shadow),
            );
            if tier == CULLED {
                continue;
            }
            let (palette, scroll) = pose_vehicle(
                units,
                scratch,
                &mut f.palettes,
                name,
                &Articulation::default(),
                None,
            );
            models.push(Model {
                tint: art.tint(side),
                palette,
                scroll,
                ..Model::unit(art.row(name, "", NONE), tier, pass, t.position, t.heading)
            });
        }
        // Corpses lying, and those the cap pushed out sinking away.
        let lying = poses
            .corpses
            .iter()
            .map(|c| (c, 0.0, false))
            .chain(poses.fading.iter().map(|f| (&f.corpse, f.sink, true)));
        for (c, sink, fading) in lying {
            let Some(name) =
                units.resolve(&c.kind, &c.side, c.soldier, c.slot, c.operator_mount, None)
            else {
                continue;
            };
            if fading {
                f.counts.fading += 1;
            } else {
                f.counts.corpses += 1;
            }
            let at = [c.position[0], c.position[1], c.position[2] - sink];
            let size = bounds(units, name, 1 + c.lies);
            let tier = model_detail(
                &detail,
                view,
                at,
                size.size,
                size.radius,
                Carries::Card,
                None,
            );
            if tier == CULLED {
                continue;
            }
            let row = art.row(name, "", c.lies as u32);
            let Some(palette) = corpse_palettes.append(row, &mut f.palettes, || {
                posing::rest_palette(units, tables, inverse_binds, name, c.lies as u32)
            }) else {
                continue;
            };
            // Remains are seen or unseen whole, as the ground under them.
            models.push(Model {
                fog: model_fog(true, false, false),
                tint: art.tint(&c.side),
                palette,
                ..Model::unit(row, tier, 0, at, c.yaw)
            });
        }
        // A cook-off's moving wreck and its thrown debris; a piece in
        // motion is never a card, and reaches as far as it has moved.
        let looks = &input.effects.wreck_looks;
        pieces.clear();
        cook_offs.models(
            &poses.vehicles,
            time,
            units,
            looks,
            &effects.presentation().cook_off,
            pieces,
        );
        for piece in pieces.iter() {
            let size = match piece.state {
                Some(_) => looks
                    .values()
                    .find(|l| *l.appearance == *piece.appearance)
                    .map_or_else(Bounds::default, |l| bounds_size(l.bounds)),
                None => bounds(units, &piece.appearance, 0),
            };
            let grow = piece.scale[0].max(piece.scale[1]).max(piece.scale[2]);
            let moved = piece.motion.map_or(0.0, |m| {
                grow * (m[12] * m[12] + m[13] * m[13] + m[14] * m[14]).sqrt()
            });
            let tier = model_detail(
                &detail,
                view,
                piece.position,
                size.size * grow,
                size.radius * grow + moved,
                Carries::Nothing,
                None,
            );
            if tier == CULLED {
                continue;
            }
            let (palette, scroll) = match (&piece.articulation, &piece.motion) {
                (Some(a), motion) => pose_vehicle(
                    units,
                    scratch,
                    &mut f.palettes,
                    &piece.appearance,
                    a,
                    motion.as_ref(),
                ),
                (None, Some(m)) => {
                    f.palettes.push(joint(&skin::affine(m)));
                    (f.palettes.len() as u32 - 1, [0.0; 2])
                }
                (None, None) => (0, [0.0; 2]),
            };
            let row = art.row(&piece.appearance, piece.state.unwrap_or(""), NONE);
            models.push(Model {
                tint: piece.tint.map_or([1.0; 3], |t| t.map(|v| v as f32)),
                palette,
                scroll,
                scale: piece.scale.map(|v| v as f32),
                // A thrown piece is the world's; the whole wreck still moving
                // is posed as a unit is.
                fog: model_fog(false, piece.state.is_some(), false),
                ..Model::unit(row, tier, 0, piece.position, piece.yaw)
            });
        }
        // Ghosts: where the UI's units will stand, at rest, in their colour.
        for g in ui.ghosts {
            let Some(_) = units.types.get(&g.kind) else {
                continue;
            };
            let Some(name) = units.resolve(&g.kind, side, g.id, g.slot, None, None) else {
                continue;
            };
            let at = [g.x, g.y, g.z];
            let far = bounds(units, name, 0);
            let tier = model_detail(
                &detail,
                view,
                at,
                far.size,
                far.radius,
                Carries::Nothing,
                None,
            );
            if tier == CULLED {
                continue;
            }
            let (palette, scroll) = match units.appearances.get(name).map(AsRef::as_ref) {
                Some(Appearance::Skinned { far_pose, .. }) => (
                    pose_body(
                        units,
                        tables,
                        inverse_binds,
                        &mut scratch.worlds,
                        &mut f.palettes,
                        name,
                        &far_pose.clip,
                        far_pose.phase,
                        None,
                        Kick::default(),
                    ),
                    [0.0; 2],
                ),
                _ => pose_vehicle(
                    units,
                    scratch,
                    &mut f.palettes,
                    name,
                    &Articulation::default(),
                    None,
                ),
            };
            let colour = match g.look {
                1 => xray.selected,
                2 => input.blocked,
                _ => xray.own,
            };
            models.push(Model {
                colour,
                palette,
                scroll,
                ..Model::unit(art.row(name, "", NONE), tier, GHOST_PASS, at, g.yaw)
            });
        }
        f.counts.culled = culled;
        picks::picks(sample, units, &body, f);
        emitters(o, poses, side, units, f);
    }
}

impl Presentation {
    /// What to draw at presentation time `time` for `camera`, for a renderer
    /// at `generation`: a number it bumps whenever it holds no static batches
    /// (a new GPU frame, a new world, a jump), so a new one is written every
    /// static batch. The camera's view, its uniform and the sun's cascades
    /// come with it, and the scenery has followed the ground the side has
    /// learned cleared by the publications pushed so far. Errs when the side
    /// knows a building in a state its template lacks.
    pub fn frame(
        &mut self,
        time: f64,
        camera: &Camera,
        ui: &Ui,
        generation: u32,
    ) -> Result<&Frame, String> {
        self.know(ui.known)?;
        self.camera = Some(*camera);
        let f = &mut self.frame;
        f.index += 1;
        f.clock = time;
        f.batches.clear();
        f.instances.clear();
        f.palettes.clear();
        f.palettes.push(IDENTITY);
        f.picks.clear();
        f.emitters.clear();
        f.counts = Counts::default();
        f.light_wrap = 0.0;
        let view = View::new(camera);
        f.view = view.record(camera);
        cameras::cameras(f, &view, &mut self.shadow, &self.ground, camera, time);
        self.follow_felled(time);
        self.follow_cleared();
        // A wreck the cook-off draws moving is left out of the props until
        // it lies still.
        if let (Some(props), Some(loaded)) = (&mut self.props, &self.loaded) {
            if props.set_moving(loaded.cook_offs.moving_props().collect()) {
                self.statics.rewalk();
            }
        }
        let sample = self.interpolator.frame(time);
        // A body the side watches shoved moves every frame, as units do.
        if let Some(props) = &mut self.props {
            if props.set_shoved(sample.map_or(&[], |s| &s.shoved)) {
                self.statics.rewalk();
            }
        }
        let mut scenery = match (&mut self.scenery, &self.ground) {
            (Some(walk), Some(ground)) if !walk.rows.is_empty() => {
                Some(SceneryPopulation { walk, ground })
            }
            _ => None,
        };
        let mut populations =
            static_frame::populations(&mut self.buildings, &mut scenery, &mut self.props);
        let f = &mut self.frame;
        self.statics
            .write(&mut populations, camera, &view, generation, f);
        self.models.clear();
        let mut corpse_palettes = posing::CorpsePalettes::default();
        if let (Some(loaded), Some(sample)) = (self.loaded.as_mut(), sample) {
            f.tick = sample.observation.tick as u32;
            loaded.frame(
                sample,
                &view,
                ui,
                &self.world,
                self.ground.as_ref(),
                self.detail_scale[0],
                f,
                &mut self.models,
                &mut corpse_palettes,
            );
        }
        self.lab.append(
            &view,
            &self.world,
            &mut self.frame,
            &mut self.models,
            &mut corpse_palettes,
        );
        if let Some(showcase) = &mut self.showcase {
            showcase.pose(time, self.ground.as_ref());
            showcase.draw(
                &view,
                &self.world,
                self.detail_scale[0],
                &mut self.frame,
                &mut self.models,
            );
        }
        batching::write(&mut self.models, &mut self.frame);
        Ok(&self.frame)
    }
}
