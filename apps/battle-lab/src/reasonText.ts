// The labs' words for a weapon's action reason. The player's panel shows a
// reason only as its mark (`REASON_MARK`, `infoPanel.tsx`), never as words.

/** Every sim action reason in player words; none names a hidden obstacle. */
export const REASON_TEXT: Record<string, string> = {
  firing: "firing",
  no_compatible_target: "no target it can hurt",
  holding_fire: "holding fire",
  out_of_range: "out of range",
  blocked_trajectory: "no clear shot",
  friendly_in_line: "friendly vehicle in the way",
  aiming: "aiming",
  reloading: "reloading",
  turret_traversing: "turning turret",
  moving_stationary_weapon: "must stop to use",
  out_of_ammo: "out of ammunition",
  tracking_last_sighting: "tracking last sighting",
  guiding: "guiding a missile",
  no_own_sight: "needs a clear line of sight to the target",
  no_facing_slot: "no firing position facing the target",
  changing_position: "entering or leaving a building",
};
