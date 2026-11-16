use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::tap_targets, "When this creature enters, tap target permanent.").targets(&[targets::PERMANENT]),
        ability(Cost::mana(mana("{2}{U}")), effects::loot, "{2}{U}: Draw a card, then discard a card."),
    ],
    ..oracle::HARRIER_STRIX
};
