//! A skirmish player's row: its seat index, its id (an index into the
//! layout's playerIds table), its controller, whether it is ready, its
//! credits, its passive income per minute, its unit count and its unit cap.
use super::tags::tag;
use contract::players::ControllerKind;
use contract::skirmish::PlayerView;

pub(super) const PLAYER_FIELDS: [&str; 8] = [
    "index",
    "id",
    "controller",
    "ready",
    "credits",
    "incomePerMinute",
    "unitCount",
    "cap",
];

pub(super) fn player_row(player: &PlayerView) -> [f32; 8] {
    [
        f32::from(player.index.0),
        // The browser resolves this index through the layout's playerIds table.
        player.index.0 as f32,
        tag(
            &[
                ControllerKind::Human,
                ControllerKind::ActiveAi,
                ControllerKind::PassiveAi,
            ],
            &player.controller,
        ),
        player.ready as u8 as f32,
        player.credits as f32,
        player.income_per_minute as f32,
        player.unit_count as f32,
        player.cap as f32,
    ]
}
