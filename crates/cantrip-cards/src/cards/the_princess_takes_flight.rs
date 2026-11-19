use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, effects::exile_first_target_with_this, "I \u{2014} Exile up to one target creature.").targets(&[targets::CREATURE.optional()]),
        chapter(2, effects::target_creature_gets_and_gains_flying::<2, 2>, "II \u{2014} Target creature you control gets +2/+2 and gains flying until end of turn.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        chapter(3, effects::return_exiled_to_battlefield, "III \u{2014} Return the exiled card to the battlefield under its owner's control."),
    ],
    ..oracle::THE_PRINCESS_TAKES_FLIGHT
};
