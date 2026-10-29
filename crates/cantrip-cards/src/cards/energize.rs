use cantrip_api::*;
use cantrip_mechanics::{beam_me_up, effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::CREATURE_AN_OPPONENT_CONTROLS],
        effects::return_first_target_to_hand,
    ),
    ways: &[beam_me_up::way(mana("{3}{U}"), "Beam me up {3}{U}")],
    ..oracle::ENERGIZE
};
