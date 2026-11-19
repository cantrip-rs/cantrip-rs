use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::SPELL_YOU_CAST_DURING_AN_OPPONENTS_TURN,
        effects::target_creature_gets::<-1, -1>,
        "Whenever you cast a spell during an opponent's turn, up to one target creature an opponent controls gets -1/-1 until end of turn.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS.optional()])],
    ..oracle::DREAM_SPOILERS
};
