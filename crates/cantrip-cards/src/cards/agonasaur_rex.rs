use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::cycling(mana("{2}{G}"), "Cycling {2}{G} ({2}{G}, Discard this card: Draw a card.)"),
        triggered(
            triggers::YOU_CYCLE_THIS_CARD,
            effects::plus_one_counters_on_first_target_that_gains_until_end_of_turn::<2, { Keywords::of(&[Keyword::Trample, Keyword::Indestructible]).0 }>,
            "When you cycle this card, put two +1/+1 counters on up to one target creature or Vehicle. It gains trample and indestructible until end of turn.",
        )
        .targets(&[targets::CREATURE_OR_VEHICLE.between(0, 1)]),
    ],
    ..oracle::AGONASAUR_REX
};
