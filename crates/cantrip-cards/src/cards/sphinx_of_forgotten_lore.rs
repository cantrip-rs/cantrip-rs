use cantrip_api::*;
use cantrip_mechanics::{grant_flashback, targets, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        grant_flashback,
        "Whenever this creature attacks, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to that card's mana cost.",
    )
    .targets(&[targets::INSTANT_OR_SORCERY_IN_YOUR_GRAVEYARD])],
    ..oracle::SPHINX_OF_FORGOTTEN_LORE
};
