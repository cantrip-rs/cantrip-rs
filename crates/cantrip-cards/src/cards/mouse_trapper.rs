use cantrip_api::*;
use cantrip_mechanics::triggers::VALIANT;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Flash is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        VALIANT,
        effects::tap_targets,
        "Valiant \u{2014} Whenever this creature becomes the target of a spell or ability you control for the first time each turn, tap target creature an opponent controls.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::MOUSE_TRAPPER
};
