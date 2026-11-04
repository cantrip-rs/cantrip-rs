use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{3}, Sacrifice this creature: Add {R}{R}{R}." (CR 605.1a).
pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            cost("{3}").and(SACRIFICE_THIS),
            add_n(Mana::R, 3),
            "{3}, Sacrifice this creature: Add {R}{R}{R}.",
        ),
    ],
    ..oracle::COAL_GOLEM
};
