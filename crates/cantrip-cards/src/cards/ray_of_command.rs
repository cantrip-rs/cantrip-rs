use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::CREATURE_AN_OPPONENT_CONTROLS],
        effects::untap_and_borrow_tapping_it_when_lost,
    ),
    delayed: &[
        triggered(
            triggers::YOU_LOSE_CONTROL_OF_THIS,
            effects::tap_subject,
            "When you lose control of the creature, tap it.",
        ),
    ],
    ..oracle::RAY_OF_COMMAND
};
