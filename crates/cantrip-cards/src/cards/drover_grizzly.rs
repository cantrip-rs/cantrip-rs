use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS_WHILE_SADDLED;
use cantrip_mechanics::{effects, saddle};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks while saddled, creatures you control gain trample until end of turn" (CR 611.2c, 702.19): the
/// creatures its controller controls as the ability resolves, the Grizzly included. "Saddle 1" (CR 702.171a) is the shared mechanic.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS_WHILE_SADDLED,
            effects::creatures_you_control_gain_until_end_of_turn::<{ Keywords::of(&[Keyword::Trample]).0 }>,
            "Whenever this creature attacks while saddled, creatures you control gain trample until end of turn.",
        ),
        saddle::ability(1, "Saddle 1 (Tap any number of other creatures you control with total power 1 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::DROVER_GRIZZLY
};
