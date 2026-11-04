use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{6}").and(SACRIFICE_THIS),
        library::shuffle_first_target_into_owners_library,
        "{6}, Sacrifice this creature: Choose target creature or planeswalker an opponent controls. Its owner shuffles it into their library.",
    )
    .targets(&[targets::CREATURE_OR_PLANESWALKER_AN_OPPONENT_CONTROLS])],
    ..oracle::LIVING_LIBRARY
};
