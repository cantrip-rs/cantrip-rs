use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

static MODES: ModalDef = modes(&[
    mode(effects::target_creature_gets::<2, 0>, "Target creature you control gets +2/+0 until end of turn.")
        .targets(&[targets::CREATURE_YOU_CONTROL]),
    mode(
        effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Menace, Keyword::Haste]).0 }>,
        "Target creature you control gains menace and haste until end of turn.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL]),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        triggers::BEGINNING_OF_YOUR_COMBAT,
        &MODES,
        "At the beginning of combat on your turn, choose one \u{2014}\n\u{2022} Target creature you control gets +2/+0 until end of turn.\n\u{2022} Target creature you control gains menace and haste until end of turn.",
    )],
    ..oracle::FEROCIFICATION
};
