use cantrip_api::*;
use cantrip_mechanics::triggers::{self, THIS_ENTERS_OR_ATTACKS};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS_OR_ATTACKS,
            effects::exile_target_card_and_gain_life::<1>,
            "Whenever this creature enters or attacks, exile up to one target card from a graveyard. You gain 1 life.",
        )
        .targets(&[targets::CARD_IN_A_GRAVEYARD.optional()]),
        triggers::cycling(mana("{2}"), "Cycling {2} ({2}, Discard this card: Draw a card.)"),
    ],
    ..oracle::WRECK_REMOVER
};
