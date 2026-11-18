use cantrip_api::*;
use cantrip_mechanics::{effects, targets::filters};
use cantrip_oracle as oracle;

/// "{T}, Sacrifice another creature or artifact: Each opponent loses 1 life and you gain 1 life" (CR 118.3, 701.21, 119.3): the
/// sacrifice is a cost, paid with the {T}, and "another" leaves the Acolyte itself out.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP.and(sacrifice(1, filters::another_creature_or_artifact)),
        effects::each_opponent_loses_1_life_you_gain_1,
        "{T}, Sacrifice another creature or artifact: Each opponent loses 1 life and you gain 1 life.",
    )],
    ..oracle::ACOLYTE_OF_ACLAZOTZ
};
