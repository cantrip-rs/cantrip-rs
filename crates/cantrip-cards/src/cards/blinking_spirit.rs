use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "{0}: Return this creature to its owner's hand." (CR 602.1).
pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            Cost::mana(ManaCost::ZERO),
            effects::return_this_to_hand,
            "{0}: Return this creature to its owner's hand.",
        ),
    ],
    ..oracle::BLINKING_SPIRIT
};
