//! Logical rows decoded into the complete client observation.
use super::*;

impl Reader {
    /// Splits the groups into rows, reads the fog and the ground, and maps
    /// every row to its view.
    pub(super) fn frame(
        &self,
        words: &[u32],
        data: &[f32],
        groups: &[Vec<u32>],
        tail: usize,
        base: &Baseline,
    ) -> Result<Observation> {
        let h = &data[..HEADER.len()];
        let floats: Vec<Vec<f32>> = groups
            .iter()
            .map(|g| g.iter().map(|&w| f32::from_bits(w)).collect())
            .collect();
        let mut rows: Vec<Vec<Row>> = Vec::with_capacity(groups.len());
        for (g, layout) in GROUP_LAYOUTS.iter().enumerate() {
            let payload = &floats[g][..];
            let count = h[HEADER.iter().position(|n| *n == layout.count).unwrap()] as f64;
            check(
                whole(count) && count * layout.fields.len() as f64 <= payload.len() as f64,
                "observation group row count exceeds its payload",
            )?;
            rows.push(split_rows(layout, payload, count as usize)?);
        }

        let grid = [
            f!(h, HEADER, "fogNx"),
            f!(h, HEADER, "fogNy"),
            f!(h, HEADER, "fogCellM"),
        ];
        let [nx, ny, cell_m] = grid;
        let cells = if whole(nx) && whole(ny) {
            nx * ny
        } else {
            f64::INFINITY
        };
        let fog_words = (cells / 32.0).ceil();
        check(
            fog_words <= self.max_fog_words as f64 && (cells == 0.0 || cell_m > 0.0),
            "fog field dimensions exceed the admitted delivery bound",
        )?;
        let (cells, fog_words) = (cells as usize, fog_words as usize);
        let (count, fog_full) = (f!(h, HEADER, "fogFloats"), f!(h, HEADER, "fogFull"));
        check(
            fog_full == 0.0 || fog_full == 1.0,
            "unknown fog payload encoding",
        )?;
        let full = fog_full == 1.0;
        check(
            whole(count)
                && if full {
                    count == (fog_words * 2) as f64
                } else {
                    (count as usize).is_multiple_of(3) && count <= (fog_words * 2) as f64
                },
            "fog payload does not match its encoding",
        )?;
        let previous = base.fog.as_ref();
        check(
            (full || previous.is_some()) && previous.is_none_or(|p| [p.nx, p.ny, p.cell_m] == grid),
            "fog delta has no matching field baseline",
        )?;
        let count = count as usize;
        let runs = f!(h, HEADER, "groundRunCount");
        check(whole(runs), "ground runs do not match their count")?;
        let logical = (HEADER.len() + groups.iter().map(Vec::len).sum::<usize>() + count) as f64
            + runs * GROUND_FIELDS.len() as f64;
        check(
            tail + count <= data.len() && logical <= self.max_words as f64,
            "publication length does not match its layout",
        )?;
        let fog = match previous {
            Some(p) if !full && count == 0 => p.clone(),
            _ => {
                let mut bits = if full {
                    vec![0; fog_words]
                } else {
                    previous.unwrap().bits.clone()
                };
                let word = |at: usize| -> Result<u32> {
                    let (lo, hi) = (data[at] as f64, data[at + 1] as f64);
                    check(
                        whole(lo) && whole(hi) && lo < LIMB && hi < LIMB,
                        "fog word limbs must be exact 16-bit integers",
                    )?;
                    Ok(lo as u32 | (hi as u32) << LIMB_BITS)
                };
                if full {
                    for (i, bit) in bits.iter_mut().enumerate() {
                        *bit = word(tail + i * 2)?;
                    }
                } else {
                    let mut last = -1.0;
                    for at in (tail..tail + count).step_by(3) {
                        let index = data[at] as f64;
                        check(
                            is_int(index) && index > last && index < fog_words as f64,
                            "fog delta word indices must be ordered and inside the field",
                        )?;
                        bits[index as usize] = word(at + 1)?;
                        last = index;
                    }
                }
                check(
                    cells % 32 == 0 || bits[fog_words - 1] >> (cells % 32) == 0,
                    "fog padding bits must be zero",
                )?;
                Arc::new(Fog {
                    cell_m,
                    nx,
                    ny,
                    bits,
                })
            }
        };
        let runs = self.ground_runs(&words[tail + count..], runs as usize)?;

        let kept = |g: usize| base.statics.is_some() && base.groups.get(g) == Some(&groups[g]);
        let statics = base.statics.as_ref();
        let (corpses, props, fallen) =
            (group("corpses"), group("knownProps"), group("fallenBodies"));
        let skirmish = if f!(h, HEADER, "skirmishPhase") < 0.0 {
            None
        } else {
            let pending = map(&rows[group("pendingPurchases")], |r| self.pending(r))?;
            let mut players = map(&rows[group("players")], |r| self.player(r))?;
            for purchase in pending {
                if let Some(player) = players.iter_mut().find(|p| p.index == purchase.owner) {
                    player.pending.push(purchase);
                } else {
                    return Err("pending purchase names an unpublished player".into());
                }
            }
            for (owner, offer) in map(&rows[group("offers")], |r| self.offer(r))? {
                match players.iter_mut().find(|p| p.index == owner) {
                    Some(Player {
                        offered: Some(offered),
                        ..
                    }) => offered.push(offer),
                    _ => return Err("an offer names a player whose shop is not narrowed".into()),
                }
            }
            let reported =
                f!(h, HEADER, "reportTotalsCount") > 0.0 || f!(h, HEADER, "reportEventCount") > 0.0;
            Some(Skirmish {
                phase: fixed(
                    &["preparation", "active", "finished"],
                    f!(h, HEADER, "skirmishPhase"),
                ),
                players,
                preparation_remaining_s: f!(h, HEADER, "preparationRemainingS"),
                scores: [f!(h, HEADER, "scoreBlue"), f!(h, HEADER, "scoreRed")],
                result: optional_fixed(&["blue", "red", "draw"], f!(h, HEADER, "matchResult")),
                objectives: map(&rows[group("objectives")], |r| self.objective(r))?,
                report: if reported {
                    Some(Report {
                        totals: map(&rows[group("reportTotals")], |r| Ok(totals(r)))?,
                        events: map(&rows[group("reportEvents")], |r| self.event(r))?,
                    })
                } else {
                    None
                },
            })
        };
        let encounter = f!(h, HEADER, "encounterResult");
        Ok(Observation {
            tick: f!(h, HEADER, "tick"),
            own: map(&rows[group("own")], |r| self.own(r))?,
            identified: map(&rows[group("identified")], |r| self.identified(r))?,
            contacts: map(&rows[group("contacts")], |r| self.contact(r))?,
            audible: map(&rows[group("audible")], |r| self.cue(r))?,
            refusals: map(&rows[group("refusals")], |r| self.refusal(r))?,
            known_props: match statics {
                Some((_, kept_props, _)) if kept(props) => kept_props.clone(),
                _ => Arc::new(map(&rows[props], |r| self.known_prop(r))?),
            },
            projectiles: map(&rows[group("projectiles")], |r| self.projectile(r))?,
            blasts: map(&rows[group("blasts")], |r| self.blast(r))?,
            air_transits: map(&rows[group("airTransits")], |r| air_transit(self, r))?,
            corpses: match statics {
                Some((kept_corpses, _, _)) if kept(corpses) => kept_corpses.clone(),
                _ => Arc::new(map(&rows[corpses], |r| self.corpse(r))?),
            },
            fallen_bodies: match statics {
                Some((_, _, kept_fallen)) if kept(fallen) => kept_fallen.clone(),
                _ => Arc::new(map(&rows[fallen], |r| Ok(fallen_body(r)))?),
            },
            crashes: map(&rows[group("crashes")], |r| self.crash(r))?,
            shouts: map(&rows[group("shouts")], |r| self.shout(r))?,
            guided: map(&rows[group("guided")], |r| Ok(guided(r)))?,
            encounter: if encounter < 0.0 {
                None
            } else {
                Some(Encounter {
                    held_s: f!(h, HEADER, "encounterHeldS"),
                    result: self.name("encounterResults", encounter)?,
                })
            },
            skirmish,
            scenario: if f!(h, HEADER, "scenarioChapter") < 0.0 {
                None
            } else {
                let chapter = f!(h, HEADER, "scenarioChapter") as usize;
                Some(Scenario {
                    chapter: self
                        .tables
                        .get("chapters")
                        .and_then(|t| t.get(chapter))
                        .cloned(),
                    beats: map(&rows[group("beats")], |r| {
                        const FIELDS: &[&str] = &crate::publication::BEAT_FIELDS;
                        let f = r.fields;
                        Ok(ScenarioBeat {
                            beat: self.name("beats", v!(f, "beat"))?,
                            tick: v!(f, "firedLo") + v!(f, "firedHi") * LIMB,
                        })
                    })?,
                })
            },
            fog,
            ground_patch: GroundPatch {
                epoch: f!(h, HEADER, "groundEpoch"),
                side: self.name("sides", f!(h, HEADER, "groundSide"))?,
                base_revision: f!(h, HEADER, "groundBase"),
                revision: f!(h, HEADER, "groundRevision"),
                full: f!(h, HEADER, "groundFull") == 1.0,
                runs,
            },
            prop_display_poses: map(&rows[group("propDisplayPoses")], |r| {
                Ok(prop_display_pose(r))
            })?,
            grinds: map(&rows[group("grinds")], |r| Ok(grind(r)))?,
            smoke_clouds: map(&rows[group("smokeClouds")], |r| self.smoke_cloud(r))?,
            flares: map(&rows[group("flares")], |r| {
                const FIELDS: &[&str] = &FLARE_FIELDS;
                Ok(Flare {
                    from: [v!(r.fields, "ax"), v!(r.fields, "ay"), v!(r.fields, "az")],
                    to: [v!(r.fields, "bx"), v!(r.fields, "by"), v!(r.fields, "bz")],
                })
            })?,
        })
    }

    pub(super) fn leans(&self, flat: &[f32]) -> Result<Vec<Option<MemberLean>>> {
        const FIELDS: &[&str] = &LEAN_FIELDS;
        each::<{ LEAN_FIELDS.len() }>(flat)
            .map(|p| {
                let side = self.optional("leanSides", v!(p, "side"))?;
                Ok(side.map(|side| MemberLean {
                    side,
                    at: [v!(p, "x"), v!(p, "y")],
                }))
            })
            .collect()
    }

    pub(super) fn own(&self, r: &Row) -> Result<OwnUnit> {
        const FIELDS: &[&str] = &OWN_FIELDS;
        let f = r.fields;
        // Sections in the writer's order (`GROUP_LAYOUTS`).
        let [route, steps, members, member_hp, member_ids, orders, leans, sees, mounts, poses, eyes] =
            r.sections[..]
        else {
            unreachable!("own rows carry eleven sections")
        };
        let charges = v!(f, "protectionCharges");
        let orders = each::<{ MEMBER_ORDER_FIELDS.len() }>(orders).map(|p| {
            const FIELDS: &[&str] = &MEMBER_ORDER_FIELDS;
            Ok(MemberOrder {
                spot: [v!(p, "x"), v!(p, "y")],
                cover_now: self.optional("coverTiers", v!(p, "coverNow"))?,
                cover_there: self.optional("coverTiers", v!(p, "coverThere"))?,
            })
        });
        Ok(OwnUnit {
            withdrawing: v!(f, "withdrawing") == 1.0,
            fuel_s: present(v!(f, "fuelSeconds")),
            afterburner: v!(f, "afterburner") == 1.0,
            in_dogfight: v!(f, "inDogfight") == 1.0,
            flares: (v!(f, "flareCharges") >= 0.0).then(|| Protection {
                charges: v!(f, "flareCharges"),
                cooldown: present(v!(f, "flareCooldown")),
            }),
            id: v!(f, "id"),
            owner: present(v!(f, "owner")),
            kind: self.name("unitKinds", v!(f, "kind"))?,
            position: [v!(f, "x"), v!(f, "y"), v!(f, "z")],
            yaw: v!(f, "yaw"),
            goal: (!v!(f, "goalX").is_nan()).then(|| [v!(f, "goalX"), v!(f, "goalY")]),
            policy: self.optional("policies", v!(f, "policy"))?,
            direction: self.optional("directions", v!(f, "direction"))?,
            reversing: v!(f, "reversing") == 1.0,
            protection: (charges >= 0.0).then(|| Protection {
                charges,
                cooldown: present(v!(f, "protectionCooldownProgress")),
            }),
            state: self.name("moveStates", v!(f, "state"))?,
            blocker: present(v!(f, "blocker")),
            route: points(route),
            steps: each::<{ STEP_FIELDS.len() }>(steps)
                .map(|p| self.step(p))
                .collect::<Result<_>>()?,
            members: points(members),
            member_ids: member_column(member_ids, |p| limbs(p[0] as f64, p[1] as f64)),
            member_slots: member_column(member_ids, |p| p[2] as f64),
            member_active_mounts: member_column(member_ids, |p| present(p[3] as f64)),
            member_orders: orders.collect::<Result<_>>()?,
            member_leans: self.leans(leans)?,
            area: (!v!(f, "areaM").is_nan()).then(|| SquadArea {
                anchor: [v!(f, "areaX"), v!(f, "areaY")],
                radius: v!(f, "areaM"),
            }),
            final_facing: v!(f, "finalFacing"),
            sees: column(sees),
            engagement: self.name("engagements", v!(f, "engagement"))?,
            mounts: each::<{ MOUNT_FIELDS.len() }>(mounts)
                .map(|p| self.mount(p))
                .collect::<Result<_>>()?,
            weapon_poses: weapon_poses(poses),
            hp: v!(f, "hp"),
            smoking: v!(f, "smoking") == 1.0,
            shot_at: v!(f, "shotAt") == 1.0,
            smoke: present(v!(f, "smokeCharges")),
            member_hp: column(member_hp),
            suppression: self.name("suppressionTiers", v!(f, "suppression"))?,
            concealed: v!(f, "concealed") == 1.0,
            deployment: self.deployment(v!(f, "deployProgress"), v!(f, "deployTarget"))?,
            grounded: v!(f, "grounded") == 1.0,
            hold: self.hold(f)?,
            stock: present(v!(f, "stock")),
            service: self.name("serviceStatuses", v!(f, "service"))?,
            sight: Sight {
                eyes: points(eyes),
                forward: v!(f, "sightForward"),
                shape: SightShape {
                    front: v!(f, "sightFront"),
                    side: v!(f, "sightSide"),
                    rear: v!(f, "sightRear"),
                },
                range: v!(f, "sightRange"),
            },
        })
    }

    pub(super) fn refusal(&self, r: &Row) -> Result<Refusal> {
        const FIELDS: &[&str] = &REFUSAL_FIELDS;
        Ok(Refusal {
            unit: limbs(v!(r.fields, "unitLo"), v!(r.fields, "unitHi")).unwrap_or(0.0),
            reason: self.name("orderErrors", v!(r.fields, "reason"))?,
        })
    }

    pub(super) fn contact(&self, r: &Row) -> Result<Contact> {
        const FIELDS: &[&str] = &CONTACT_FIELDS;
        let f = r.fields;
        // Bit k of the heard mask, 24 rows a word.
        let mask = [v!(f, "heardLow"), v!(f, "heardMid"), v!(f, "heardHigh")]
            .into_iter()
            .enumerate()
            .fold(0_u64, |mask, (w, word)| {
                mask | (word as u64) << (w * HEARD_WORD_BITS)
            });
        let kinds = &self.tables["roundKinds"];
        Ok(Contact {
            id: v!(f, "id"),
            source: self.name("contactSources", v!(f, "source"))?,
            center: [v!(f, "x"), v!(f, "y"), v!(f, "z")],
            layer: self.name("layers", v!(f, "layer"))?,
            aloft: v!(f, "aloft") == 1.0,
            radius: v!(f, "radius"),
            evidence_tick: v!(f, "evidenceTick"),
            expires_tick: v!(f, "expiresTick"),
            kind: self.optional("unitKinds", v!(f, "kind"))?,
            heard: (0..kinds.len())
                .filter(|&k| mask >> k & 1 == 1)
                .map(|k| kinds[k].clone())
                .collect(),
            bearing_deg: present(v!(f, "bearingDeg")),
            range_m: present(v!(f, "rangeM")),
        })
    }

    pub(super) fn cue(&self, r: &Row) -> Result<SoundCue> {
        const FIELDS: &[&str] = &AUDIBLE_FIELDS;
        let f = r.fields;
        Ok(SoundCue {
            listener: v!(f, "listener"),
            category: self.name("soundCategories", v!(f, "category"))?,
            sector: v!(f, "sector"),
            band: self.name("soundBands", v!(f, "band"))?,
            moving: v!(f, "moving") == 1.0,
        })
    }

    pub(super) fn known_prop(&self, r: &Row) -> Result<KnownProp> {
        const FIELDS: &[&str] = &KNOWN_PROP_FIELDS;
        let f = r.fields;
        Ok(KnownProp {
            kind: self.name("propKinds", v!(f, "kind"))?,
            center: [v!(f, "x"), v!(f, "y")],
            yaw: v!(f, "yaw"),
            half: [v!(f, "hx"), v!(f, "hy"), v!(f, "hz")],
            base_z: v!(f, "baseZ"),
            replaces: limbs(v!(f, "replacesLo"), v!(f, "replacesHi")),
            id: limbs(v!(f, "idLo"), v!(f, "idHi")),
            building: limbs(v!(f, "buildingLo"), v!(f, "buildingHi")),
            structure_owner: limbs(v!(f, "structureOwnerLo"), v!(f, "structureOwnerHi")),
            authored_prop: limbs(v!(f, "authoredPropLo"), v!(f, "authoredPropHi")),
            destroyed: v!(f, "destroyed") == 1.0,
            wreck_of: self.optional("unitKinds", v!(f, "wreckOf"))?,
        })
    }

    pub(super) fn projectile(&self, r: &Row) -> Result<Projectile> {
        const FIELDS: &[&str] = &PROJECTILE_FIELDS;
        let f = r.fields;
        let hit = self.name("hitKinds", v!(f, "hit"))?;
        Ok(Projectile {
            path: points(r.sections[0]),
            ricochets: each::<4>(r.sections[1])
                .map(|p| Ricochet {
                    point: p[0] as f64,
                    normal: [p[1], p[2], p[3]].map(f64::from),
                })
                .collect(),
            own: v!(f, "own") == 1.0,
            kind: self.name("roundKinds", v!(f, "kind"))?,
            shooter_member: limbs(v!(f, "shooterLo"), v!(f, "shooterHi")),
            impact_normal: (&*hit != "none").then(|| [v!(f, "nx"), v!(f, "ny"), v!(f, "nz")]),
            hit,
        })
    }

    pub(super) fn blast(&self, r: &Row) -> Result<Blast> {
        const FIELDS: &[&str] = &BLAST_FIELDS;
        let f = r.fields;
        Ok(Blast {
            point: [v!(f, "x"), v!(f, "y"), v!(f, "z")],
            radius: v!(f, "radius"),
            kind: self.name("roundKinds", v!(f, "kind"))?,
            hit: self.name("hitKinds", v!(f, "hit"))?,
        })
    }

    pub(super) fn corpse(&self, r: &Row) -> Result<Corpse> {
        const FIELDS: &[&str] = &CORPSE_FIELDS;
        let f = r.fields;
        Ok(Corpse {
            position: [v!(f, "x"), v!(f, "y"), v!(f, "z")],
            own: v!(f, "own") == 1.0,
            soldier: limbs(v!(f, "soldierLo"), v!(f, "soldierHi")),
            kind: self.name("unitKinds", v!(f, "kind"))?,
            slot: v!(f, "slot"),
            yaw: v!(f, "yaw"),
        })
    }

    pub(super) fn shout(&self, r: &Row) -> Result<Shout> {
        const FIELDS: &[&str] = &SHOUT_FIELDS;
        let f = r.fields;
        Ok(Shout {
            own: v!(f, "own") == 1.0,
            id: v!(f, "idLo") + v!(f, "idHi") * LIMB,
            kind: self.name("unitKinds", v!(f, "kind"))?,
            position: [v!(f, "x"), v!(f, "y"), v!(f, "z")],
            trigger: self.name("shoutTriggers", v!(f, "trigger"))?,
            intensity: self.name("intensities", v!(f, "intensity"))?,
        })
    }
}
