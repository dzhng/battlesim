//! The admitted weapon and mount table fixed for a battle.
use super::*;

/// A weapon row with its validated flight profile.
pub struct Weapon {
    /// The row's id in the rules' `weapons`.
    pub id: String,
    pub def: WeaponDefinition,
    pub profile: LaunchProfile,
}

/// An authored mount: ammunition kinds (weapon indices) sharing targeting.
#[derive(Clone)]
pub struct MountSpec {
    pub kinds: Vec<usize>,
    pub squad: bool,
    pub turret: bool,
    /// A squad's slots whose soldiers carry it; empty on a hull.
    pub carriers: Vec<usize>,
    /// It passes to the next living soldier when its carrier falls.
    pub special: bool,
    /// The mount (index in the type's list) whose turret carries this one;
    /// `None` is the hull.
    pub on: Option<usize>,
    /// Where it turns, in the hull's frame at rest (forward, left, up).
    pub pivot: V3,
    /// The ring its carrier turns round, on the ground under the carrier's
    /// pivot: its pivot swings round this with the carrier. The hull's
    /// middle for a hull mount.
    pub ring: V3,
    /// Its muzzle from the pivot along its own bearing; `None` is a hand
    /// weapon, fired at the infantry muzzle height.
    pub muzzle: Option<V3>,
    /// The bearings a turret swings through off the hull's heading; `None`,
    /// all round.
    pub arc: Option<Arc>,
}

/// Every weapon and mount in the battle, fixed at setup.
pub struct Arsenal {
    pub weapons: Vec<Weapon>,
    /// Each unit type's mounts, by type index.
    mounts: Vec<Vec<MountSpec>>,
    /// Each unit type's optics mount (`sensors.on`), by type index.
    optics: Vec<Option<usize>>,
    pub config: FlightConfig,
}

impl Arsenal {
    pub fn new(rules: &Rules) -> Self {
        let config = FlightConfig::new(&rules.physics.flight, rules.tick_hz);
        debug_assert!(config.is_ok(), "flight configuration was admitted");
        let config = config.unwrap_or_else(|e| unreachable!("admitted flight configuration: {e}"));
        let weapons: Vec<Weapon> = rules
            .weapons
            .iter()
            .map(|(id, def)| Weapon {
                id: id.clone(),
                profile: config
                    .profile(&def.ballistics)
                    .unwrap_or_else(|e| unreachable!("admitted weapon flight: {e}")),
                def: def.clone(),
            })
            .collect();
        // The catalog's checks at load hold every mount to an existing row
        // and an earlier turret (`Catalog::check_weapons`, `catalog::check`).
        let index = |id: &str| {
            weapons
                .iter()
                .position(|w| w.id == id)
                .expect("mounts name weapon rows (checked at load)")
        };
        let catalog = &rules.catalog;
        let mounts: Vec<Vec<MountSpec>> = catalog
            .indices()
            .map(|t| {
                let list = catalog.mounts(t);
                list.iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let m = &c.def;
                        let on = m.on.as_ref().map(|carrier| {
                            list[..i]
                                .iter()
                                .position(|c| c.def.id == *carrier && c.def.turret)
                                .expect("a mount rides an earlier turret (checked at load)")
                        });
                        let v = |[x, y, z]: [f64; 3]| v3(x, y, z);
                        let ring = on.map_or(V3::default(), |c| {
                            let [x, y, _] = list[c].def.pivot_m;
                            v3(x, y, 0.0)
                        });
                        MountSpec {
                            kinds: m.weapons.iter().map(|w| index(w)).collect(),
                            squad: m.squad,
                            turret: m.turret,
                            carriers: c.carriers.clone(),
                            special: m.special,
                            on,
                            pivot: v(m.pivot_m),
                            ring,
                            muzzle: m.muzzle_m.map(v),
                            arc: m.arc.map(|a| Arc {
                                centre: a.centre_deg.to_radians(),
                                half: (a.width_deg / 2.0).to_radians(),
                            }),
                        }
                    })
                    .collect()
            })
            .collect();
        let optics = catalog
            .indices()
            .map(|t| {
                let on = catalog.get(t).sensors.on.as_ref()?;
                catalog.mounts(t).iter().position(|m| &m.def.id == on)
            })
            .collect();
        Arsenal {
            weapons,
            mounts,
            optics,
            config,
        }
    }

    /// A mount's report as a firing report hears it: a bit for every row it
    /// fires (the layout's `roundKinds`), since its sound doesn't say which.
    pub fn heard(&self, kind: TypeIndex, mount: usize) -> u64 {
        self.mounts[kind.0 as usize][mount]
            .kinds
            .iter()
            .fold(0, |m, &k| m | 1u64 << k)
    }

    pub fn specs(&self, kind: TypeIndex) -> &[MountSpec] {
        &self.mounts[kind.0 as usize]
    }

    /// The mount a type's optics turn with; `None`, its hull.
    pub fn optics(&self, kind: TypeIndex) -> Option<usize> {
        self.optics[kind.0 as usize]
    }

    /// Fresh mounts for a unit of type `kind` facing `yaw`, loaded and aimed nowhere.
    pub fn mounts_for(
        &self,
        kind: TypeIndex,
        yaw: f64,
        members: &[crate::units::Soldier],
    ) -> Vec<Mount> {
        self.specs(kind)
            .iter()
            .enumerate()
            .map(|(i, spec)| Mount {
                spec: i,
                ammo: spec
                    .kinds
                    .iter()
                    .map(|&k| match self.weapons[k].def.ammo {
                        AmmoCapacity::Unlimited(_) => None,
                        AmmoCapacity::Rounds(n) => Some(n),
                    })
                    .collect(),
                cycles: if spec.squad {
                    members
                        .iter()
                        .filter(|s| spec.carriers.contains(&s.slot))
                        .map(|s| Cycle::new(Some(s.id), spec, &self.weapons))
                        .collect()
                } else {
                    vec![Cycle::new(None, spec, &self.weapons)]
                },
                operator: (!spec.squad)
                    .then(|| {
                        members
                            .iter()
                            .find(|s| s.alive() && spec.carriers.contains(&s.slot))
                            .map(|s| s.id)
                    })
                    .flatten(),
                lock: None,
                support: None,
                bearing: yaw,
                last_enemy_lock_tick: None,
                elevation: 0.0,
                shots: 0,
                reason: ActionReason::NoCompatibleTarget,
            })
            .collect()
    }
}
