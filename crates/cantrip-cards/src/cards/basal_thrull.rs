use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}, Sacrifice this creature: Add {B}{B}." (CR 605.1a).
pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            SACRIFICE_THIS.and(TAP),
            add_n(Mana::B, 2),
            "{T}, Sacrifice this creature: Add {B}{B}.",
        ),
    ],
    ..oracle::BASAL_THRULL
};
