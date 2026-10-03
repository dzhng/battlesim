# Complete generation parameter reference

Working snapshot, 2026-10-02. Read [the generation report](generation-report.md) for the algorithms and acceptance roles. This appendix is the requested exhaustive data reference, not another settings owner. It will be refreshed at final identity integration.

The preset data below comes from the **unintegrated continuous-coverage candidate** at `city-ground-fix` (merged main, dirty source). Its version labels remain `layout-12` / `layout-presets-11` pending the final coordinated bump. The current selected 50% median visibility gate lives in the test/report contract, not this preset file.

Preset input SHA-256: `77a3ba1d715463c03a3879b4bd122f6273a0be8ab0a61ad0e8cadf29e16a91a2`.

Production’s former `open_country.sight` used reach 480 m, scatter 0.5, 30 attempts and copse/tree-line weights 4/1. The replacement below retains 100 m location cells, tries copse then tree line, and derives its reach from actual physical observers; those deleted controls are not future workbench knobs.

## Reading values

- `_m`, `_m2`, `_km`, `_ha`, `_s`, `_deg` and `_kmh` specify units. One hectare is 10,000 m². Unmarked rotations in this preset are radians.
- Two-entry proposal ranges mean minimum/maximum, not X/Y unless the field is explicitly a dimension pair. Integer count endpoints are inclusive; random fractional draws exclude their upper endpoint.
- Probability/share values use 0–1. Category and body `weight`/`mix` values are normalized relative weights. `half_extents_m` are half sizes in X/Y/Z.
- A missing optional district/class row has its schema/default behavior; absence does not imply a user-tunable zero. The narrative report explains the active rule.
- These numbers describe proposals, geometry, limits and diagnostics with different roles; changing a target does not override physical legality or a hard user requirement.

## Presets: revision

| Setting | Value |
| --- | --- |
| `revision` | `"layout-presets-11"` |

## Presets: terrain

| Setting | Value |
| --- | --- |
| `fog_cell_m` | `8` |
| `height_grid_m` | `4` |
| `slope_cutoff_deg` | `35` |

## Presets: approach

| Setting | Value |
| --- | --- |
| `depth_m` | `1800` |
| `front_m` | `400` |
| `reserve_front_m` | `650` |
| `reserve_margin_m` | `120` |
| `bearing_candidates` | `16` |

## Presets: fairness

| Setting | Value |
| --- | --- |
| `town.rel` | `0.1` |
| `town.abs` | `0.004` |
| `forest.rel` | `0.2` |
| `forest.abs` | `0.005` |
| `river.rel` | `0.2` |
| `river.abs` | `0.05` |

## Presets: transit

| Setting | Value |
| --- | --- |
| `road_kmh` | `110` |
| `allowance_s` | `15` |
| `max_s` | `215` |
| `exit_window` | `0.3` |
| `centre_reach_m` | `500` |

## Presets: roads

| Setting | Value |
| --- | --- |
| `country_road_width_m` | `8` |
| `dirt_track_width_m` | `4` |
| `dirt_track_speed_factor` | `0.75` |
| `bend_step_m` | `300` |
| `bend_amplitude` | `0.05` |
| `bend_max_m` | `150` |
| `hub_offset_m` | `[450, 80]` |
| `cross_road_chance` | `0.5` |
| `side_road_chance` | `0.6` |
| `side_exit_window` | `0.7` |
| `junction_reach_m` | `1500` |
| `junction_chance` | `0.6` |
| `junction_apart_m` | `700` |
| `waypoint_reach_m` | `400` |
| `main_street_reach` | `0.9` |
| `gate_margin_m` | `30` |
| `turn_max_deg` | `50` |
| `through_reach` | `0.25` |
| `link_max_m` | `3500` |
| `corridors_max` | `4` |

## Presets: sites

| Setting | Value |
| --- | --- |
| `edge_margin_m` | `80` |
| `cluster_reach_m` | `1200` |

## Presets: towns

| Setting | Value |
| --- | --- |
| `avenue_width_m` | `10` |
| `growth_noise` | `0.6` |
| `growth_patch_blocks` | `3` |
| `growth_patches_min` | `3` |
| `corner_min_deg` | `50` |
| `block_split` | `[0.4, 0.6]` |
| `block_skew_deg` | `8` |
| `road_frontage` | `2` |
| `align_m` | `30` |
| `junction_clear_m` | `100` |
| `park_reach_blocks` | `8` |
| `park_min_ha` | `0.3` |

## Presets: forests

| Setting | Value |
| --- | --- |
| `large_radius_m` | `[450, 950]` |
| `small_radius_m` | `[110, 340]` |
| `large_chance` | `[0.15, 0.6]` |
| `min_radius_m` | `60` |
| `aspect` | `[0.4, 1]` |
| `outline_noise` | `0.35` |
| `outline_points` | `36` |
| `settlement_gap_m` | `25` |
| `forest_gap_m` | `40` |
| `share_tolerance` | `0.005` |
| `max_woods` | `80` |
| `infill_chance` | `0.5` |
| `infill_cover` | `[0.5, 1.6]` |

## Presets: rivers

| Setting | Value |
| --- | --- |
| `width_m` | `[16, 38]` |
| `width_swing` | `0.15` |
| `bank_grade` | `0.25` |
| `freeboard_m` | `1.2` |
| `point_turn_deg` | `8` |
| `point_step_m` | `16` |
| `meander.wavelength_m` | `[900, 1800]` |
| `meander.amplitude` | `[0.1, 0.18]` |
| `meander.overtone_gain` | `0.15` |
| `meander.overtone_ratio` | `[1.6, 2]` |
| `side_margin_m` | `500` |
| `settlement_gap_m` | `40` |
| `forest_gap_m` | `10` |
| `road_gap_m` | `22` |
| `junction_gap_m` | `150` |
| `bridge.deck` | `"bridge_deck"` |
| `bridge.width_m` | `14` |
| `bridge.landing_m` | `9` |
| `bridge.deck_z` | `0.1` |
| `bridge.thickness_m` | `0.8` |
| `bridge.approach_m` | `12` |
| `bridge.span_max_m` | `90` |
| `bridge.skew_max_deg` | `45` |
| `bridge.reuse_m.country_road` | `600` |
| `bridge.reuse_m.dirt_track` | `1200` |
| `bridge.worth_m` | `800` |

## Presets: retries

| Setting | Value |
| --- | --- |
| `centre` | `60` |
| `site` | `200` |
| `forest` | `60` |
| `river` | `150` |
| `repair_settlements` | `4` |
| `repair_woods` | `5` |
| `repair_blocks` | `4` |
| `infill` | `6` |
| `fit` | `3` |

## Presets: parcels

| Setting | Value |
| --- | --- |
| `regional_families` | `["china"]` |
| `prop_kind` | `"building"` |
| `street_width_m` | `7` |
| `verge_m` | `2` |
| `street_step_m` | `45` |
| `lot_step_m` | `4` |
| `run_on_m` | `60` |
| `tail_min_m` | `35` |
| `junction_clear_m` | `60` |

## Presets: street_props

| Setting | Value |
| --- | --- |
| `lane_margin_m` | `0.9` |
| `kerb_gap_m` | `0.3` |
| `wall_gap_m` | `0.9` |
| `door_clear_m` | `3` |
| `corner_clear_m` | `8` |
| `slide_m` | `6` |
| `slide_step_m` | `0.25` |
| `attempts` | `6` |
| `bodies.parked_car.half_extents_m` | `[2.1, 0.9, 0.75]` |
| `bodies.parked_car.clear_m` | `1.5` |
| `bodies.lamp.half_extents_m` | `[0.15, 0.15, 3]` |
| `bodies.lamp.clear_m` | `1` |
| `bodies.street_tree.half_extents_m` | `[0.35, 0.35, 5]` |
| `bodies.street_tree.clear_m` | `1` |
| `bodies.bus_shelter.half_extents_m` | `[2, 0.75, 1.3]` |
| `bodies.bus_shelter.clear_m` | `1.5` |
| `bodies.bench.half_extents_m` | `[0.9, 0.3, 0.45]` |
| `bodies.bench.clear_m` | `1` |
| `bodies.bins.half_extents_m` | `[0.6, 0.4, 0.6]` |
| `bodies.bins.clear_m` | `1` |
| `bodies.planter.half_extents_m` | `[0.8, 0.4, 0.4]` |
| `bodies.planter.clear_m` | `1` |
| `bodies.bollard.half_extents_m` | `[0.15, 0.15, 0.5]` |
| `bodies.bollard.clear_m` | `1` |
| `bodies.hydrant.half_extents_m` | `[0.2, 0.2, 0.4]` |
| `bodies.hydrant.clear_m` | `1` |
| `bodies.utility_box.half_extents_m` | `[0.6, 0.3, 0.7]` |
| `bodies.utility_box.clear_m` | `1` |
| `bodies.scooter.half_extents_m` | `[0.8, 0.3, 0.55]` |
| `bodies.scooter.clear_m` | `1` |
| `bodies.skip_bin.half_extents_m` | `[1.8, 0.9, 0.7]` |
| `bodies.skip_bin.clear_m` | `1` |
| `bodies.pallet_stack.half_extents_m` | `[0.6, 0.5, 0.6]` |
| `bodies.pallet_stack.clear_m` | `0.6` |
| `bodies.site_cabin.half_extents_m` | `[3, 1.2, 1.3]` |
| `bodies.site_cabin.clear_m` | `1` |
| `bodies.heras_fence.half_extents_m` | `[1.75, 0.05, 1]` |
| `bodies.heras_fence.clear_m` | `0` |
| `parking.kind` | `"parked_car"` |
| `parking.run` | `[2, 5]` |
| `parking.bumper_gap_m` | `0.5` |
| `parking.run_gap_m` | `3` |
| `parking.both_sides_min_width_m` | `10` |
| `site.max_per_settlement` | `3` |
| `site.min_lot_m` | `[14, 14]` |
| `site.cabin` | `"site_cabin"` |
| `site.fence` | `"heras_fence"` |
| `site.fence_inset_m` | `1` |
| `site.gate_m` | `4` |
| `site.stock[0].kind` | `"skip_bin"` |
| `site.stock[0].count` | `[1, 1]` |
| `site.stock[1].kind` | `"pallet_stack"` |
| `site.stock[1].count` | `[2, 5]` |

## Presets: districts

| Setting | Value |
| --- | --- |
| `farm.mix.farmstead` | `80` |
| `farm.mix.detached_home` | `20` |
| `farm.ground_m` | `[60, 55]` |
| `farm.streets.surface` | `"dirt_track"` |
| `farm.streets.block_depth_m` | `150` |
| `farm.streets.block_length_m` | `220` |
| `farm.streets.bend.amplitude_m` | `10` |
| `farm.streets.bend.wavelength_m` | `300` |
| `farm.streets.cross_skip` | `0.3` |
| `farm.lots.front_m` | `8` |
| `farm.lots.side_m` | `8` |
| `farm.lots.rear_m` | `8` |
| `farm.lots.coverage` | `0.85` |
| `farm.lots.apron_m` | `0` |
| `village.mix.detached_home` | `85` |
| `village.mix.farmstead` | `15` |
| `village.ground_m` | `[55, 55]` |
| `village.streets.surface` | `"road"` |
| `village.streets.block_depth_m` | `96` |
| `village.streets.block_length_m` | `160` |
| `village.streets.bend.amplitude_m` | `14` |
| `village.streets.bend.wavelength_m` | `320` |
| `village.streets.cross_skip` | `0.2` |
| `village.lots.front_m` | `6` |
| `village.lots.side_m` | `6` |
| `village.lots.rear_m` | `10` |
| `village.lots.coverage` | `0.85` |
| `village.lots.apron_m` | `0` |
| `village.props.parking` | `0.08` |
| `village.props.verge[0].kind` | `"lamp"` |
| `village.props.verge[0].spacing_m` | `120` |
| `village.props.verge[0].sides` | `"alternate"` |
| `village.props.verge[1].kind` | `"bench"` |
| `village.props.verge[1].spacing_m` | `400` |
| `village.props.verge[1].sides` | `"scatter"` |
| `garden_suburb.mix.detached_home` | `90` |
| `garden_suburb.mix.attached_home` | `10` |
| `garden_suburb.ground_m` | `[50, 45]` |
| `garden_suburb.streets.surface` | `"road"` |
| `garden_suburb.streets.block_depth_m` | `84` |
| `garden_suburb.streets.block_length_m` | `140` |
| `garden_suburb.streets.bend.amplitude_m` | `16` |
| `garden_suburb.streets.bend.wavelength_m` | `360` |
| `garden_suburb.streets.cross_skip` | `0.15` |
| `garden_suburb.lots.front_m` | `6` |
| `garden_suburb.lots.side_m` | `6` |
| `garden_suburb.lots.rear_m` | `10` |
| `garden_suburb.lots.coverage` | `0.85` |
| `garden_suburb.lots.apron_m` | `0` |
| `garden_suburb.props.parking` | `0.12` |
| `garden_suburb.props.verge[0].kind` | `"lamp"` |
| `garden_suburb.props.verge[0].spacing_m` | `90` |
| `garden_suburb.props.verge[0].sides` | `"alternate"` |
| `garden_suburb.props.verge[1].kind` | `"hydrant"` |
| `garden_suburb.props.verge[1].spacing_m` | `300` |
| `garden_suburb.props.verge[1].sides` | `"scatter"` |
| `garden_suburb.props.verge[2].kind` | `"utility_box"` |
| `garden_suburb.props.verge[2].spacing_m` | `300` |
| `garden_suburb.props.verge[2].sides` | `"scatter"` |
| `small_centre.mix.attached_home` | `85` |
| `small_centre.mix.detached_home` | `15` |
| `small_centre.ground_m` | `[60, 60]` |
| `small_centre.streets.surface` | `"road"` |
| `small_centre.streets.block_depth_m` | `70` |
| `small_centre.streets.block_length_m` | `130` |
| `small_centre.streets.cross_skip` | `0` |
| `small_centre.lots.front_m` | `2` |
| `small_centre.lots.side_m` | `1.5` |
| `small_centre.lots.rear_m` | `6` |
| `small_centre.lots.coverage` | `0.95` |
| `small_centre.lots.apron_m` | `0` |
| `small_centre.props.parking` | `0.35` |
| `small_centre.props.verge[0].kind` | `"lamp"` |
| `small_centre.props.verge[0].spacing_m` | `70` |
| `small_centre.props.verge[0].sides` | `"alternate"` |
| `small_centre.props.verge[1].kind` | `"bins"` |
| `small_centre.props.verge[1].spacing_m` | `200` |
| `small_centre.props.verge[1].sides` | `"scatter"` |
| `small_centre.props.verge[2].kind` | `"bench"` |
| `small_centre.props.verge[2].spacing_m` | `250` |
| `small_centre.props.verge[2].sides` | `"scatter"` |
| `small_centre.props.verge[3].kind` | `"planter"` |
| `small_centre.props.verge[3].spacing_m` | `250` |
| `small_centre.props.verge[3].sides` | `"scatter"` |
| `small_centre.props.verge[4].kind` | `"bollard"` |
| `small_centre.props.verge[4].spacing_m` | `300` |
| `small_centre.props.verge[4].sides` | `"scatter"` |
| `small_centre.props.verge[5].kind` | `"hydrant"` |
| `small_centre.props.verge[5].spacing_m` | `300` |
| `small_centre.props.verge[5].sides` | `"scatter"` |
| `small_centre.props.verge[6].kind` | `"utility_box"` |
| `small_centre.props.verge[6].spacing_m` | `300` |
| `small_centre.props.verge[6].sides` | `"scatter"` |
| `small_centre.props.verge[7].kind` | `"scooter"` |
| `small_centre.props.verge[7].spacing_m` | `300` |
| `small_centre.props.verge[7].sides` | `"scatter"` |
| `centre.mix.attached_home` | `85` |
| `centre.mix.urban_apartment` | `15` |
| `centre.ground_m` | `[60, 60]` |
| `centre.streets.surface` | `"road"` |
| `centre.streets.block_depth_m` | `72` |
| `centre.streets.block_length_m` | `140` |
| `centre.streets.cross_skip` | `0` |
| `centre.lots.front_m` | `2` |
| `centre.lots.side_m` | `1.5` |
| `centre.lots.rear_m` | `6` |
| `centre.lots.coverage` | `0.95` |
| `centre.lots.apron_m` | `0` |
| `centre.props.parking` | `0.38` |
| `centre.props.verge[0].kind` | `"lamp"` |
| `centre.props.verge[0].spacing_m` | `60` |
| `centre.props.verge[0].sides` | `"alternate"` |
| `centre.props.verge[1].kind` | `"street_tree"` |
| `centre.props.verge[1].spacing_m` | `24` |
| `centre.props.verge[1].sides` | `"both"` |
| `centre.props.verge[1].avenue` | `true` |
| `centre.props.verge[2].kind` | `"bus_shelter"` |
| `centre.props.verge[2].spacing_m` | `500` |
| `centre.props.verge[2].sides` | `"scatter"` |
| `centre.props.verge[2].avenue` | `true` |
| `centre.props.verge[3].kind` | `"bins"` |
| `centre.props.verge[3].spacing_m` | `180` |
| `centre.props.verge[3].sides` | `"scatter"` |
| `centre.props.verge[4].kind` | `"bench"` |
| `centre.props.verge[4].spacing_m` | `220` |
| `centre.props.verge[4].sides` | `"scatter"` |
| `centre.props.verge[5].kind` | `"planter"` |
| `centre.props.verge[5].spacing_m` | `220` |
| `centre.props.verge[5].sides` | `"scatter"` |
| `centre.props.verge[6].kind` | `"bollard"` |
| `centre.props.verge[6].spacing_m` | `300` |
| `centre.props.verge[6].sides` | `"scatter"` |
| `centre.props.verge[7].kind` | `"hydrant"` |
| `centre.props.verge[7].spacing_m` | `300` |
| `centre.props.verge[7].sides` | `"scatter"` |
| `centre.props.verge[8].kind` | `"utility_box"` |
| `centre.props.verge[8].spacing_m` | `260` |
| `centre.props.verge[8].sides` | `"scatter"` |
| `centre.props.verge[9].kind` | `"scooter"` |
| `centre.props.verge[9].spacing_m` | `260` |
| `centre.props.verge[9].sides` | `"scatter"` |
| `centre.props.site_chance` | `0.15` |
| `apartments.mix.urban_apartment` | `85` |
| `apartments.mix.attached_home` | `15` |
| `apartments.ground_m` | `[90, 80]` |
| `apartments.streets.surface` | `"road"` |
| `apartments.streets.block_depth_m` | `130` |
| `apartments.streets.block_length_m` | `230` |
| `apartments.streets.cross_skip` | `0.2` |
| `apartments.lots.front_m` | `16` |
| `apartments.lots.side_m` | `10` |
| `apartments.lots.rear_m` | `14` |
| `apartments.lots.coverage` | `0.9` |
| `apartments.lots.apron_m` | `12` |
| `apartments.props.parking` | `0.33` |
| `apartments.props.verge[0].kind` | `"lamp"` |
| `apartments.props.verge[0].spacing_m` | `70` |
| `apartments.props.verge[0].sides` | `"alternate"` |
| `apartments.props.verge[1].kind` | `"street_tree"` |
| `apartments.props.verge[1].spacing_m` | `26` |
| `apartments.props.verge[1].sides` | `"both"` |
| `apartments.props.verge[1].avenue` | `true` |
| `apartments.props.verge[2].kind` | `"bus_shelter"` |
| `apartments.props.verge[2].spacing_m` | `500` |
| `apartments.props.verge[2].sides` | `"scatter"` |
| `apartments.props.verge[2].avenue` | `true` |
| `apartments.props.verge[3].kind` | `"bins"` |
| `apartments.props.verge[3].spacing_m` | `220` |
| `apartments.props.verge[3].sides` | `"scatter"` |
| `apartments.props.verge[4].kind` | `"bench"` |
| `apartments.props.verge[4].spacing_m` | `220` |
| `apartments.props.verge[4].sides` | `"scatter"` |
| `apartments.props.verge[5].kind` | `"hydrant"` |
| `apartments.props.verge[5].spacing_m` | `300` |
| `apartments.props.verge[5].sides` | `"scatter"` |
| `apartments.props.verge[6].kind` | `"utility_box"` |
| `apartments.props.verge[6].spacing_m` | `260` |
| `apartments.props.verge[6].sides` | `"scatter"` |
| `apartments.props.verge[7].kind` | `"scooter"` |
| `apartments.props.verge[7].spacing_m` | `300` |
| `apartments.props.verge[7].sides` | `"scatter"` |
| `apartments.props.site_chance` | `0.15` |
| `core.mix.urban_apartment` | `85` |
| `core.mix.highrise` | `15` |
| `core.ground_m` | `[80, 70]` |
| `core.streets.surface` | `"road"` |
| `core.streets.block_depth_m` | `120` |
| `core.streets.block_length_m` | `150` |
| `core.streets.cross_skip` | `0` |
| `core.lots.front_m` | `10` |
| `core.lots.side_m` | `9` |
| `core.lots.rear_m` | `10` |
| `core.lots.coverage` | `0.9` |
| `core.lots.apron_m` | `0` |
| `core.props.parking` | `0.38` |
| `core.props.verge[0].kind` | `"lamp"` |
| `core.props.verge[0].spacing_m` | `60` |
| `core.props.verge[0].sides` | `"alternate"` |
| `core.props.verge[1].kind` | `"street_tree"` |
| `core.props.verge[1].spacing_m` | `24` |
| `core.props.verge[1].sides` | `"both"` |
| `core.props.verge[1].avenue` | `true` |
| `core.props.verge[2].kind` | `"bus_shelter"` |
| `core.props.verge[2].spacing_m` | `400` |
| `core.props.verge[2].sides` | `"scatter"` |
| `core.props.verge[2].avenue` | `true` |
| `core.props.verge[3].kind` | `"bins"` |
| `core.props.verge[3].spacing_m` | `160` |
| `core.props.verge[3].sides` | `"scatter"` |
| `core.props.verge[4].kind` | `"bench"` |
| `core.props.verge[4].spacing_m` | `200` |
| `core.props.verge[4].sides` | `"scatter"` |
| `core.props.verge[5].kind` | `"planter"` |
| `core.props.verge[5].spacing_m` | `200` |
| `core.props.verge[5].sides` | `"scatter"` |
| `core.props.verge[6].kind` | `"bollard"` |
| `core.props.verge[6].spacing_m` | `260` |
| `core.props.verge[6].sides` | `"scatter"` |
| `core.props.verge[7].kind` | `"hydrant"` |
| `core.props.verge[7].spacing_m` | `280` |
| `core.props.verge[7].sides` | `"scatter"` |
| `core.props.verge[8].kind` | `"utility_box"` |
| `core.props.verge[8].spacing_m` | `240` |
| `core.props.verge[8].sides` | `"scatter"` |
| `core.props.verge[9].kind` | `"scooter"` |
| `core.props.verge[9].spacing_m` | `240` |
| `core.props.verge[9].sides` | `"scatter"` |
| `core.props.site_chance` | `0.2` |
| `industrial.mix.industry` | `100` |
| `industrial.ground_m` | `[130, 90]` |
| `industrial.streets.surface` | `"road"` |
| `industrial.streets.block_depth_m` | `210` |
| `industrial.streets.block_length_m` | `280` |
| `industrial.streets.cross_skip` | `0.3` |
| `industrial.lots.front_m` | `30` |
| `industrial.lots.side_m` | `12` |
| `industrial.lots.rear_m` | `10` |
| `industrial.lots.coverage` | `0.85` |
| `industrial.lots.apron_m` | `30` |
| `industrial.props.parking` | `0.05` |
| `industrial.props.verge[0].kind` | `"lamp"` |
| `industrial.props.verge[0].spacing_m` | `100` |
| `industrial.props.verge[0].sides` | `"alternate"` |
| `industrial.props.verge[1].kind` | `"hydrant"` |
| `industrial.props.verge[1].spacing_m` | `300` |
| `industrial.props.verge[1].sides` | `"scatter"` |
| `industrial.props.verge[2].kind` | `"utility_box"` |
| `industrial.props.verge[2].spacing_m` | `240` |
| `industrial.props.verge[2].sides` | `"scatter"` |
| `industrial.props.yard[0].kind` | `"skip_bin"` |
| `industrial.props.yard[0].count` | `[0, 2]` |
| `industrial.props.yard[1].kind` | `"pallet_stack"` |
| `industrial.props.yard[1].count` | `[1, 4]` |

## Presets: classes

| Setting | Value |
| --- | --- |
| `hamlet.rank` | `0` |
| `hamlet.area_ha` | `[5, 11]` |
| `hamlet.aspect` | `[0.5, 0.8]` |
| `hamlet.rotation` | `[0, 3.1416]` |
| `hamlet.outline.exponent` | `2` |
| `hamlet.outline.noise` | `0.25` |
| `hamlet.outline.points` | `10` |
| `hamlet.road` | `"dirt_track"` |
| `hamlet.approach` | `false` |
| `hamlet.built_share` | `[0.35, 0.5]` |
| `hamlet.block.depth_m` | `[65, 85]` |
| `hamlet.block.length_m` | `[70, 150]` |
| `hamlet.ribbon` | `3` |
| `hamlet.neighbourhood` | `[1, 3]` |
| `hamlet.zones[0].to` | `1` |
| `hamlet.zones[0].districts.farm` | `1` |
| `village.rank` | `1` |
| `village.area_ha` | `[18, 46]` |
| `village.aspect` | `[0.4, 0.75]` |
| `village.rotation` | `[0, 3.1416]` |
| `village.outline.exponent` | `2` |
| `village.outline.noise` | `0.25` |
| `village.outline.points` | `10` |
| `village.road` | `"country_road"` |
| `village.approach` | `true` |
| `village.built_share` | `[0.45, 0.65]` |
| `village.block.depth_m` | `[70, 130]` |
| `village.block.length_m` | `[150, 360]` |
| `village.ribbon` | `2.5` |
| `village.neighbourhood` | `[1, 3]` |
| `village.zones[0].to` | `1` |
| `village.zones[0].districts.village` | `1` |
| `small_town.rank` | `2` |
| `small_town.area_ha` | `[60, 110]` |
| `small_town.aspect` | `[0.55, 0.9]` |
| `small_town.rotation` | `[0, 3.1416]` |
| `small_town.outline.exponent` | `2` |
| `small_town.outline.noise` | `0.25` |
| `small_town.outline.points` | `11` |
| `small_town.road` | `"country_road"` |
| `small_town.approach` | `true` |
| `small_town.built_share` | `[0.6, 0.75]` |
| `small_town.block.depth_m` | `[110, 200]` |
| `small_town.block.length_m` | `[160, 320]` |
| `small_town.ribbon` | `1.2` |
| `small_town.parks` | `[0, 1]` |
| `small_town.neighbourhood` | `[2, 3]` |
| `small_town.zones[0].to` | `0.2` |
| `small_town.zones[0].districts.small_centre` | `1` |
| `small_town.zones[1].to` | `1` |
| `small_town.zones[1].districts.garden_suburb` | `1` |
| `small_town.zones[1].roadside.garden_suburb` | `5` |
| `small_town.zones[1].roadside.industrial` | `1` |
| `town.rank` | `3` |
| `town.area_ha` | `[95, 210]` |
| `town.aspect` | `[0.55, 0.9]` |
| `town.rotation` | `[0, 3.1416]` |
| `town.outline.exponent` | `2` |
| `town.outline.noise` | `0.25` |
| `town.outline.points` | `11` |
| `town.road` | `"country_road"` |
| `town.approach` | `true` |
| `town.built_share` | `[0.6, 0.75]` |
| `town.block.depth_m` | `[130, 230]` |
| `town.block.length_m` | `[200, 380]` |
| `town.ribbon` | `1.2` |
| `town.parks` | `[1, 1]` |
| `town.open_blocks` | `0.06` |
| `town.neighbourhood` | `[2, 4]` |
| `town.zones[0].to` | `0.15` |
| `town.zones[0].districts.centre` | `1` |
| `town.zones[1].to` | `1` |
| `town.zones[1].districts.garden_suburb` | `6` |
| `town.zones[1].districts.apartments` | `2` |
| `town.zones[1].roadside.garden_suburb` | `5` |
| `town.zones[1].roadside.apartments` | `1` |
| `town.zones[1].roadside.industrial` | `1` |
| `large_town.rank` | `4` |
| `large_town.area_ha` | `[420, 620]` |
| `large_town.aspect` | `[0.6, 1]` |
| `large_town.rotation` | `[0, 3.1416]` |
| `large_town.outline.exponent` | `2` |
| `large_town.outline.noise` | `0.25` |
| `large_town.outline.points` | `12` |
| `large_town.road` | `"country_road"` |
| `large_town.approach` | `true` |
| `large_town.built_share` | `[0.58, 0.7]` |
| `large_town.block.depth_m` | `[200, 340]` |
| `large_town.block.length_m` | `[300, 560]` |
| `large_town.ribbon` | `1` |
| `large_town.side_roads.count` | `[1, 2]` |
| `large_town.side_roads.from` | `[0.1, 0.35]` |
| `large_town.side_roads.turn_deg` | `[60, 120]` |
| `large_town.side_roads.apart_m` | `250` |
| `large_town.parks` | `[1, 2]` |
| `large_town.open_blocks` | `0.06` |
| `large_town.neighbourhood` | `[3, 6]` |
| `large_town.zones[0].to` | `0.1` |
| `large_town.zones[0].districts.centre` | `1` |
| `large_town.zones[1].to` | `0.35` |
| `large_town.zones[1].districts.apartments` | `2` |
| `large_town.zones[1].districts.centre` | `2` |
| `large_town.zones[1].districts.garden_suburb` | `2` |
| `large_town.zones[2].to` | `1` |
| `large_town.zones[2].districts.garden_suburb` | `5` |
| `large_town.zones[2].districts.apartments` | `2` |
| `large_town.zones[2].roadside.garden_suburb` | `5` |
| `large_town.zones[2].roadside.industrial` | `1.5` |
| `city.rank` | `5` |
| `city.area_share` | `[0.16, 0.24]` |
| `city.aspect` | `[0.5, 1]` |
| `city.rotation` | `[-0.25, 0.25]` |
| `city.outline.exponent` | `3` |
| `city.outline.noise` | `0.15` |
| `city.outline.points` | `14` |
| `city.road` | `"country_road"` |
| `city.approach` | `true` |
| `city.built_share` | `[0.66, 0.8]` |
| `city.block.depth_m` | `[220, 380]` |
| `city.block.length_m` | `[320, 600]` |
| `city.ribbon` | `0.8` |
| `city.side_roads.count` | `[2, 4]` |
| `city.side_roads.from` | `[0.25, 0.6]` |
| `city.side_roads.turn_deg` | `[60, 120]` |
| `city.side_roads.apart_m` | `300` |
| `city.parks` | `[2, 3]` |
| `city.open_blocks` | `0.06` |
| `city.neighbourhood` | `[4, 8]` |
| `city.zones[0].to` | `0.06` |
| `city.zones[0].districts.core` | `1` |
| `city.zones[1].to` | `0.3` |
| `city.zones[1].districts.apartments` | `3` |
| `city.zones[1].districts.centre` | `3` |
| `city.zones[1].districts.garden_suburb` | `2` |
| `city.zones[2].to` | `1` |
| `city.zones[2].districts.garden_suburb` | `5` |
| `city.zones[2].districts.apartments` | `3` |
| `city.zones[2].districts.centre` | `1` |
| `city.zones[2].roadside.garden_suburb` | `5` |
| `city.zones[2].roadside.apartments` | `2` |
| `city.zones[2].roadside.industrial` | `1.5` |

## Presets: types

| Setting | Value |
| --- | --- |
| `open.categories` | `["farmstead", "detached_home", "attached_home", "urban_apartment", "industry"]` |
| `open.max_floors` | `6` |
| `open.centre.class` | `"small_town"` |
| `open.siting.on_road` | `0.4` |
| `open.siting.near_main` | `0.1` |
| `open.siting.beside_river` | `0.2` |
| `open.gap_m` | `500` |
| `open.forest_share` | `[0.04, 0.22]` |
| `open.river_chance` | `0.5` |
| `open.sizes.small.urban_share_max` | `0.07` |
| `open.sizes.small.settlements.village` | `[2, 3]` |
| `open.sizes.small.settlements.hamlet` | `[5, 6]` |
| `open.sizes.medium.urban_share_max` | `0.07` |
| `open.sizes.medium.settlements.village` | `[5, 7]` |
| `open.sizes.medium.settlements.hamlet` | `[9, 11]` |
| `open.sizes.large.urban_share_max` | `0.07` |
| `open.sizes.large.settlements.village` | `[9, 11]` |
| `open.sizes.large.settlements.hamlet` | `[15, 17]` |
| `mixed.categories` | `["farmstead", "detached_home", "attached_home", "urban_apartment", "industry"]` |
| `mixed.max_floors` | `8` |
| `mixed.centre.class` | `"large_town"` |
| `mixed.siting.on_road` | `0.55` |
| `mixed.siting.near_main` | `0.25` |
| `mixed.siting.beside_river` | `0.1` |
| `mixed.gap_m` | `350` |
| `mixed.forest_share` | `[0.04, 0.17]` |
| `mixed.river_chance` | `0.5` |
| `mixed.sizes.small.urban_share_max` | `0.26` |
| `mixed.sizes.small.settlements.town` | `[1, 2]` |
| `mixed.sizes.small.settlements.village` | `[2, 3]` |
| `mixed.sizes.small.settlements.hamlet` | `[3, 4]` |
| `mixed.sizes.medium.urban_share_max` | `0.2` |
| `mixed.sizes.medium.settlements.town` | `[2, 3]` |
| `mixed.sizes.medium.settlements.village` | `[4, 6]` |
| `mixed.sizes.medium.settlements.hamlet` | `[6, 7]` |
| `mixed.sizes.large.urban_share_max` | `0.16` |
| `mixed.sizes.large.settlements.town` | `[3, 4]` |
| `mixed.sizes.large.settlements.village` | `[8, 10]` |
| `mixed.sizes.large.settlements.hamlet` | `[10, 12]` |
| `metro.categories` | `["farmstead", "detached_home", "attached_home", "urban_apartment", "highrise", "industry"]` |
| `metro.centre.class` | `"city"` |
| `metro.siting.on_road` | `0.55` |
| `metro.siting.near_main` | `0.3` |
| `metro.siting.beside_river` | `0.1` |
| `metro.gap_m` | `250` |
| `metro.forest_share` | `[0.02, 0.08]` |
| `metro.river_chance` | `0.4` |
| `metro.sizes.small.urban_share_max` | `0.3` |
| `metro.sizes.small.settlements.town` | `[0, 1]` |
| `metro.sizes.small.settlements.village` | `[1, 2]` |
| `metro.sizes.small.settlements.hamlet` | `[1, 2]` |
| `metro.sizes.medium.urban_share_max` | `0.3` |
| `metro.sizes.medium.settlements.town` | `[2, 3]` |
| `metro.sizes.medium.settlements.village` | `[3, 4]` |
| `metro.sizes.medium.settlements.hamlet` | `[2, 4]` |
| `metro.sizes.large.urban_share_max` | `0.3` |
| `metro.sizes.large.settlements.town` | `[4, 5]` |
| `metro.sizes.large.settlements.village` | `[5, 7]` |
| `metro.sizes.large.settlements.hamlet` | `[5, 6]` |

## Presets: open_country

| Setting | Value |
| --- | --- |
| `sight.cell_m` | `100` |
| `sight.fill` | `["copse", "tree_line"]` |
| `clear.edge_m` | `12` |
| `clear.settlement_m` | `60` |
| `clear.forest_m` | `40` |
| `clear.road_m` | `3` |
| `clear.water_m` | `12` |
| `clear.yard_m` | `6` |
| `clear.approach_m` | `25` |
| `fairness.rel` | `0.25` |
| `fairness.homes` | `3` |
| `fairness.tree_line_m` | `250` |
| `fairness.copses` | `2` |
| `fairness.trees` | `3` |
| `fairness.cover` | `6` |
| `homesteads.per_road_km` | `0.5` |
| `homesteads.apart_m` | `260` |
| `homesteads.lane_chance` | `0.35` |
| `homesteads.lane_m` | `[60, 150]` |
| `homesteads.spread_m` | `14` |
| `homesteads.yard.front_m` | `8` |
| `homesteads.yard.side_m` | `7` |
| `homesteads.yard.rear_m` | `8` |
| `homesteads.yard.door_clear_m` | `3` |
| `homesteads.groups[0].weight` | `45` |
| `homesteads.groups[0].homes` | `[1, 1]` |
| `homesteads.groups[0].mix.farmstead` | `1` |
| `homesteads.groups[1].weight` | `55` |
| `homesteads.groups[1].homes` | `[2, 4]` |
| `homesteads.groups[1].mix.detached_home` | `85` |
| `homesteads.groups[1].mix.farmstead` | `15` |
| `homesteads.tree_chance` | `0.7` |
| `homesteads.tree_plot_m` | `[12, 16]` |
| `homesteads.body_chance` | `0.7` |
| `homesteads.bodies[0].kind` | `"parked_car"` |
| `homesteads.bodies[0].half_extents_m` | `[2.1, 0.9, 0.75]` |
| `homesteads.bodies[0].weight` | `5` |
| `homesteads.bodies[1].kind` | `"pallet_stack"` |
| `homesteads.bodies[1].half_extents_m` | `[0.6, 0.5, 0.6]` |
| `homesteads.bodies[1].weight` | `2` |
| `homesteads.bodies[2].kind` | `"crate"` |
| `homesteads.bodies[2].half_extents_m` | `[0.6, 0.6, 0.6]` |
| `homesteads.bodies[2].weight` | `2` |
| `homesteads.attempts` | `40` |
| `tree_lines.per_km2` | `0.1` |
| `tree_lines.length_m` | `[60, 130]` |
| `tree_lines.width_m` | `12` |
| `tree_lines.stretch_m` | `[45, 90]` |
| `tree_lines.gap_m` | `12` |
| `tree_lines.roadside_chance` | `0.6` |
| `tree_lines.road_gap_m` | `8` |
| `tree_lines.align_m` | `300` |
| `tree_lines.attempts` | `40` |
| `copses.per_km2` | `0.1` |
| `copses.area_m2` | `[300, 1200]` |
| `copses.aspect` | `[0.5, 1]` |
| `copses.outline_points` | `12` |
| `copses.attempts` | `40` |
| `lone_trees.per_km2` | `1.2` |
| `lone_trees.plot_m` | `9` |
| `lone_trees.attempts` | `30` |
| `field_cover.per_km2` | `1.2` |
| `field_cover.spread_m` | `7` |
| `field_cover.gap_m` | `1.2` |
| `field_cover.bodies[0].kind` | `"boulder"` |
| `field_cover.bodies[0].half_extents_m` | `[1, 0.8, 0.75]` |
| `field_cover.bodies[0].weight` | `5` |
| `field_cover.bodies[0].count` | `[1, 4]` |
| `field_cover.bodies[1].kind` | `"log"` |
| `field_cover.bodies[1].half_extents_m` | `[2.2, 0.35, 0.35]` |
| `field_cover.bodies[1].weight` | `3` |
| `field_cover.bodies[1].count` | `[2, 4]` |
| `field_cover.bodies[1].stacked` | `true` |
| `field_cover.bodies[2].kind` | `"car_wreck"` |
| `field_cover.bodies[2].half_extents_m` | `[2.1, 0.9, 0.7]` |
| `field_cover.bodies[2].weight` | `1` |
| `field_cover.bodies[3].kind` | `"pallet_stack"` |
| `field_cover.bodies[3].half_extents_m` | `[0.6, 0.5, 0.6]` |
| `field_cover.bodies[3].weight` | `1` |
| `field_cover.bodies[3].count` | `[2, 4]` |
| `field_cover.attempts` | `30` |

## Shared forest physical rule

| Setting | Value |
| --- | --- |
| `tree` | `"trunk"` |
| `log` | `"log"` |
| `boulder` | `"boulder"` |
| `rule.trunk_spacing_m` | `9` |
| `rule.trunk_jitter` | `0.3` |
| `rule.concealment_infantry` | `0.25` |
| `rule.concealment_vehicle` | `0.7` |
| `rule.attenuation_per_m` | `0.011` |
| `rule.canopy_radius_m` | `6.5` |
| `rule.canopy_height_m` | `12` |
| `rule.trunk_radius_m` | `0.35` |
| `rule.trunk_height_m` | `10` |
| `rule.trunk_clearance_m` | `2` |
| `rule.logs_per_ha` | `0` |
| `rule.boulders_per_ha` | `0` |
| `rule.log_half_extents_m` | `[2.2, 0.35, 0.35]` |
| `rule.boulder_half_extents_m` | `[1, 0.8, 0.75]` |

## Ground sight and surface rule inputs

| Setting | Value |
| --- | --- |
| `physics.infantry_eye_m` | `1.6` |
| `sensors.fog_target_height_m` | `1.0` |
| `surfaces.road.speed_factor` | `1` |
| `surfaces.country_road.speed_factor` | `1` |
| `surfaces.dirt_track.speed_factor` | `0.75` |
| `surfaces.sidewalk.speed_factor` | `0` |

## Assault encounter recipe

| Setting | Value |
| --- | --- |
| `revision` | `"encounters-1"` |
| `recipes.assault.attacker` | `"blue"` |
| `recipes.assault.attacker_edge` | `"bottom"` |
| `recipes.assault.forces.blue[0].kind` | `"jeep"` |
| `recipes.assault.forces.blue[0].post` | `"column"` |
| `recipes.assault.forces.blue[1].kind` | `"recon"` |
| `recipes.assault.forces.blue[1].post` | `"column"` |
| `recipes.assault.forces.blue[2].kind` | `"tank"` |
| `recipes.assault.forces.blue[2].post` | `"column"` |
| `recipes.assault.forces.blue[3].kind` | `"tank"` |
| `recipes.assault.forces.blue[3].post` | `"column"` |
| `recipes.assault.forces.blue[4].kind` | `"rifle"` |
| `recipes.assault.forces.blue[4].post` | `"column"` |
| `recipes.assault.forces.blue[5].kind` | `"rifle"` |
| `recipes.assault.forces.blue[5].post` | `"column"` |
| `recipes.assault.forces.blue[6].kind` | `"rifle"` |
| `recipes.assault.forces.blue[6].post` | `"column"` |
| `recipes.assault.forces.blue[7].kind` | `"at"` |
| `recipes.assault.forces.blue[7].post` | `"column"` |
| `recipes.assault.forces.blue[8].kind` | `"supply"` |
| `recipes.assault.forces.blue[8].post` | `"column"` |
| `recipes.assault.forces.red[0].kind` | `"rifle"` |
| `recipes.assault.forces.red[0].post` | `"garrison"` |
| `recipes.assault.forces.red[1].kind` | `"rifle"` |
| `recipes.assault.forces.red[1].post` | `"garrison"` |
| `recipes.assault.forces.red[2].kind` | `"rifle"` |
| `recipes.assault.forces.red[2].post` | `"garrison"` |
| `recipes.assault.forces.red[3].kind` | `"at"` |
| `recipes.assault.forces.red[3].post` | `"overwatch"` |
| `recipes.assault.forces.red[3].engagement` | `"return_fire_only"` |
| `recipes.assault.forces.red[4].kind` | `"jeep"` |
| `recipes.assault.forces.red[4].post` | `"column"` |
| `recipes.assault.forces.red[5].kind` | `"tank"` |
| `recipes.assault.forces.red[5].post` | `"column"` |
| `recipes.assault.objective.settlement` | `"main"` |
| `recipes.assault.objective.open_approach` | `"required"` |
| `recipes.assault.objective.zone_radius_m` | `150` |
| `recipes.assault.objective.hold_s` | `30` |
| `recipes.assault.objective.max_assessment_s` | `900` |
| `recipes.assault.deployment.edge_inset_m` | `150` |
| `recipes.assault.deployment.spacing_m` | `30` |
| `recipes.assault.deployment.step_m` | `50` |
| `recipes.assault.deployment.max_advance_m` | `1500` |
| `recipes.assault.deployment.pace` | `"jeep"` |
| `recipes.assault.deployment.max_route_difference_s` | `15` |
| `recipes.assault.deployment.defender_advances` | `true` |
| `recipes.assault.garrison.reach_m` | `150` |
| `recipes.assault.garrison.apart_m` | `60` |
| `recipes.assault.garrison.door_standoff_m` | `4` |
| `recipes.assault.overwatch.standoff_m` | `5` |
| `recipes.assault.overwatch.apart_m` | `25` |
| `recipes.assault.overwatch.sight_m` | `1000` |
| `recipes.assault.defender.at_attack_range_m` | `900` |
| `recipes.assault.defender.tank_retreat_hp_fraction` | `0.35` |
| `recipes.assault.defender.infantry_retreat_survivor_fraction` | `0.5` |
| `recipes.assault.clearance_m` | `1` |
| `recipes.assault.attempts.objectives` | `3` |
| `recipes.assault.attempts.roads` | `4` |
| `recipes.assault.attempts.buildings` | `40` |
| `recipes.assault.attempts.posts` | `12` |

## Source-level numeric geometry constants

The following values are fixed implementation geometry/proof settings, not currently preset sliders. Their source comments explain why they exist. They are included so the later tuning workbench can decide which are genuine business-rule controls and which should remain numerical safeguards. Mathematical constants and image-inspection palette/layout values are excluded.

### [crates/mapgen/src/joints.rs](../../crates/mapgen/src/joints.rs)

| Name | Value/expression | Purpose from source |
| --- | --- | --- |
| `MEET_M` | `0.5` | Ends this near each other are one place. |
| `OVERSHOOT_M` | `0.25` | An end stops this far past the middle of the road it joins, so the two centrelines cross whatever a centimetre of rounding did to either. |
| `NEAR_M` | `2.5` | An end joins a carriageway whose edge is this near it. |
| `REVERSE_COS` | `-0.996` | Two ends that turn back on each other this exactly (about 5°) are not welded: the shared centreline has no bend to give them. |
| `THROUGH_COS` | `0.866` | At a junction a wider road also ends on, two narrower ends within this of straight (30°) are one road through it; otherwise the wider road carries on down one of them ([`carry`]). |
| `TAIL_SPARE_M` | `1.0` | An end's last run is at least this much longer than the widest road's half width: the cut of a square end reaches half a width back, and a corner behind it is to stay round. A turn nearer an end than that is dropped, and no way's width changes that near the corner it was carried round. |
| `CARRY_ROUNDS` | `3` | A corner of unlike ways is mended over at most this many rounds: a way changes once a round. |
| `BESIDE_WIDTHS` | `10.0` | Two alike streets laid side by side whose ends run past each other by no more than this many widths are one street. |
| `JUNCTION_M` | `25.0` | Two ways that each cross a third within this of where they crossed each other are still joined there. |
| `SWING_WIDTHS` | `4.0` | An end whose last run is no longer than this many of its widths may be swung to meet a road square, when it has no room to turn before it. |
| `SLANT_COS` | `0.5` | A branch within this of square to the road it joins (30°) is left as it comes; one that comes in at more of a slant is bent to meet it square. |
| `SQUARE_WIDTHS` | `1.5` | A branch bent to meet a road square runs square for this many of its own widths before the road's edge. |
| `STUB_WIDTHS` | `2.0` | A road that stops within this many of its own widths past a road it crossed is cut back to that road: a stub, not a road of its own. |
| `FLUSH_M` | `0.02` | A face along another carriageway's edge is covered by it. |
| `REACHED_M` | `0.005` | An end is moved no less than this. |
| `ON_MIDDLE_M` | `0.1` | An end this near a road's middle stands on it, and has not passed it. |
| `ROUNDING_M` | `0.005` | How far a centimetre's rounding may move a point. |
| `ALONGSIDE_COS` | `0.94` | A road nearly alongside is not one an end runs into (20°). |
| `GATE_WIDTHS` | `2.0` | A road leaves the map square to its edge over twice its width. |
| `SQUARE_SIN` | `0.02` | Lines this near parallel (about 1°) have no corner between them. |
| `TRIM_MARGIN_M` | `1.0` | A road cut back to a settlement stops this far past the last ground it runs along, and is walked back to it in steps this long. |
| `TRIM_STEP_M` | `2.0` | See the owning source for its geometric use. |
| `PIN_ROUNDS` | `4` | How many times the ways of a lost joint are pinned and the rest closed again, before the plan's roads are left as they were laid. |
| `SHORTEST_RUN_M` | `1.0` | No end is cut back, and no gate laid, to leave a run shorter than this. |

### [crates/mapgen/src/layout/measure.rs](../../crates/mapgen/src/layout/measure.rs)

| Name | Value/expression | Purpose from source |
| --- | --- | --- |
| `OVERRUN_M` | `1.0` | A road that ends on another runs no farther than this past its middle. |

### [crates/mapgen/src/layout/roads.rs](../../crates/mapgen/src/layout/roads.rs)

| Name | Value/expression | Purpose from source |
| --- | --- | --- |
| `ON_GROUND_M` | `0.5` | A point this near a settlement's limit stands on its ground. |
| `SAME_PLACE_M` | `1.0` | Two points nearer than this are one place: no road runs between them. |
| `STRAIGHT_M` | `0.25` | A stretch of road whose points lie this near one line is straight. |
| `FORK_SIN` | `0.707` | Two roads meet no nearer alongside than this (the sine of 45 degrees). |
| `SIDE_ROAD_TRIES` | `4` | How many places are tried for each secondary road a settlement has. |

### [crates/mapgen/src/layout/towns/ground.rs](../../crates/mapgen/src/layout/towns/ground.rs)

| Name | Value/expression | Purpose from source |
| --- | --- | --- |
| `CORNER_M` | `5e-3` | Two points nearer than this are one corner: two leaves each work out where a cut crosses the edge between them, a hair apart. |
| `SNAP_M` | `0.01` | A cut that passes a corner nearer than this passes through it: the plan is on a centimetre grid, and a corner and a cut a millimetre apart would be two corners to one leaf and one to its neighbour. |
| `SLIVER_M` | `1.0` | A line must enter a piece of ground this far to cut it: nearer its edge it runs along that edge. |
| `SHARED_M` | `8.0` | A road must run this far through a piece of ground to cut it, and two pieces must share this much edge to be neighbours. |
| `SLIVER_M2` | `1.0` | A piece of ground smaller than this is no piece at all. |
| `MEET_M` | `0.5` | Two carriageways within this of each other meet. |
| `PLACES` | `6` | How many places either side of the one drawn a cut is tried at, to end clear of the junctions on the cuts it ends on. |
| `IN_LINE_SIN` | `0.423` | Two cuts that end on a third from its two sides make a crossroads when they run within this of one line (the sine of 25 degrees). |

### [crates/mapgen/src/layout/towns/streets.rs](../../crates/mapgen/src/layout/towns/streets.rs)

| Name | Value/expression | Purpose from source |
| --- | --- | --- |
| `JOIN_OVERSHOOT_M` | `0.5` | How far an avenue runs past the middle of the road it meets. |
| `SLANT_SIN` | `0.375` | An avenue meets a road no farther off square than this (the sine of 22 degrees): at more of a slant it stops at its last corner before it. |
| `ALONG_SIN` | `0.02` | A street within this of a road's line (a sine) runs along it. |

### `crates/mapgen/src/open_country/coverage.rs` (pending integration)

| Name | Value/expression | Purpose from source |
| --- | --- | --- |
| `PROOF_OPERATIONS` | `512_000_000` | Proof work only, independent of authored compile limits. Charged geometry predicates and raster reads stop at this envelope; unsupported edited spacing is refused before the shared candidate sampler runs. |

### [crates/mapgen/src/open_country/mod.rs](../../crates/mapgen/src/open_country/mod.rs)

| Name | Value/expression | Purpose from source |
| --- | --- | --- |
| `JOIN_OVERSHOOT_M` | `0.5` | A lane runs this far past the middle of the road it leaves, so the two centrelines cross whatever a centimetre of rounding did to either. |
| `LINE_STEP_M` | `20.0` | A tree line follows its road by points this far apart. |
| `WALK_M` | `50.0` | Road is measured, and a settlement's edge sampled, this often. |
| `PLACED_MAX` | `4096` | The most things of one kind a map is given, whatever its rows ask. |

### [crates/mapgen/src/parcels/streets.rs](../../crates/mapgen/src/parcels/streets.rs)

| Name | Value/expression | Purpose from source |
| --- | --- | --- |
| `JOIN_OVERSHOOT_M` | `0.5` | How far a street runs past the middle of the one it meets, so the two centrelines cross whatever a centimetre of rounding did to either. |
| `FACING_PAST_M` | `10.0` | A street runs to an end that faces it when that end is no more than this far past the first carriageway it would cross. |
| `PARALLEL_COS` | `0.866` | Two carriageways within this of parallel run the same way. |
| `FACING_COS` | `0.5` | Two streets that stop end to end run opposite ways within this (60 degrees). |
| `SLANT_SIN` | `0.342` | A street comes to a carriageway on its own line no farther off square than this (the sine of 20 degrees). |
| `TURN_SIN` | `0.47` | One that would come to it farther off square, up to this (the sine of 28 degrees), turns at its last crossing to meet it square: a bend, where a sharper turn would be a hook. At more of a slant it does not meet it. |
| `TURN_WIDTHS` | `3.0` | A street that turns to meet a carriageway square runs at least this many of its widths from the turn to the carriageway. |
| `IN_LINE_COS` | `0.906` | Two streets that meet a road from its two sides make a crossroads when they run opposite ways within this (25 degrees). |
| `AHEAD_SPREAD` | `0.36` | A street that stops in the open looks this far to either side of its line for the carriageway it stops short of (20 degrees). |
| `ROAD_EDGE` | `2.0` | How much an edge of a district counts for when the district picks the line of its streets: twice its length where a road runs along it, a quarter where no carriageway does. |
| `OPEN_EDGE` | `0.25` | See the owning source for its geometric use. |
| `ON_WAY_M` | `1.0` | A joint this near a carriageway's paving lies on it. |
| `BOW_LENGTHS` | `[1.2, 2.4]` | A bowed street's wave is this many times its district's length, or the preset's wavelength where that is longer: one bend or less from end to end, never a ripple. |
| `BOW_REACH` | `0.04` | And it swings no farther off its line than this share of that length, so a short street bends as gently as a long one. |
| `ALONE_WIDTHS` | `4.0` | A street that is one run from a crossing to the road ahead is at least this many widths long: shorter, it is a connector between two streets that already lie side by side. |
| `CROWD_WIDTHS` | `3.0` | Two streets of one grid come to a carriageway no nearer each other than this many of their widths. |
| `OWN_TAN` | `0.2` | A link runs to a node of its own grid that lies no farther off its line than this share of the way there. |
| `CORNER_TAN` | `0.47` | A street stopped in the open turns to another's end that lies no farther off its line than this share of the way to it (25 degrees). |
| `CARRIED_M` | `1.0` | A district's edge carries a carriageway whose own edge is this near it. |
| `END_MARGIN_M` | `1.0` | A street that ends in the open stops this far inside its district. |

### [crates/mapgen/src/street_props.rs](../../crates/mapgen/src/street_props.rs)

| Name | Value/expression | Purpose from source |
| --- | --- | --- |
| `EDGE_M` | `1.0` | A body keeps this far inside the map's edge. |
| `SLACK_M` | `0.05` | A body is set this far past the line it must keep to, so rounding its centre to a centimetre cannot put it over. |
| `DOOR_REACH_M` | `12.0` | A door's way to the street is this long where no carriageway lies ahead. |
| `DOOR_LOOK_M` | `40.0` | How far ahead of a door a carriageway is looked for. |
| `OWNER_STEP_M` | `2.0` | A side of a carriageway is asked whose district it is this often. |
| `SITE_ROOM_M` | `1.5` | A fence panel, a cabin and loose stock keep this far inside the fence. |

### [crates/contract/src/curve.rs](../../crates/contract/src/curve.rs)

| Name | Value/expression | Purpose from source |
| --- | --- | --- |
| `SAMPLE_SPACING_M` | `2.0` | See the owning source for its geometric use. |

### [crates/contract/src/river.rs](../../crates/contract/src/river.rs)

| Name | Value/expression | Purpose from source |
| --- | --- | --- |
| `MIN_WIDTH_CELLS` | `3.0` | A river is at least this many height samples wide, or the grid cannot carve its bed: a narrower channel falls between samples. |
| `GRID_GRADE_MARGIN` | `0.9` | A surface whose grade is `g` everywhere is drawn by grid triangles no steeper than `g √2` (a triangle's rise along each axis is one sample's). Banks keep this share of the grade that would reach the slope cutoff. |

### [crates/contract/src/forest.rs](../../crates/contract/src/forest.rs)

| Name | Value/expression | Purpose from source |
| --- | --- | --- |
| `FOLIAGE_SAMPLE_M` | `1.0` | Maximum physical foliage-depth integration step, shared by proofs. |

## Additional physical catalogues

The building catalogue supplies each template’s dimensions, parts, facade bays and legal placement frame. The resolved unit/prop catalogue supplies hulls, body properties, sight shapes and concealment used by generation. Individual asset vertex/bay coordinates and combat weapon tuning are not generation business-rule parameters; their owners remain [template geometry](../../crates/contract/src/templates.rs), [unit and prop entries](../../fixtures/README.md), and the runtime generation inputs. No duplicate catalogue is embedded here.

## Snapshot completeness

This appendix transcribes every leaf of the current preset JSON (698 rows), the complete forest rule and assault recipe, and 76 source numeric constants. Final integration must regenerate the snapshot and audit the owning logic; matching these values alone does not prove generation or gameplay.
