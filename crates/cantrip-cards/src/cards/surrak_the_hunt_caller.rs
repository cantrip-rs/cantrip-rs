use cantrip_api::*;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{effects, targets, your_creatures_have_total_power_at_least};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // Formidable's "if" is intervening (CR 603.4): checked as the beginning of combat trigger and again as it resolves.
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Haste]).0 }>,
        "Formidable \u{2014} At the beginning of combat on your turn, if creatures you control have total power 8 or greater, target creature you control gains haste until end of turn.",
    )
    .condition(your_creatures_have_total_power_at_least::<8>)
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::SURRAK_THE_HUNT_CALLER
};
