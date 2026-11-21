use cantrip_api::*;
use cantrip_mechanics::{fight, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_AN_OPPONENT_CONTROLS], fight::first_target_bites_second),
    ..oracle::ROCKY_REBUKE
};
