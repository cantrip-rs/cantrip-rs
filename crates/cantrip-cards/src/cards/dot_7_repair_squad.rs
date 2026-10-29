use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{1}{W}: Untap target creature. Activate only as a sorcery and only once each turn."
/// The core enforces the timing and per-object activation limit (CR 602.5b, 602.5d).
pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}{W}"), effects::untap_targets, "{1}{W}: Untap target creature. Activate only as a sorcery and only once each turn.")
        .targets(&[targets::CREATURE]).sorcery_speed().once_each_turn()],
    ..oracle::DOT_7_REPAIR_SQUAD
};
