use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "{1}, {T}, Pay 1 life: Create a Treasure token." (CR 602.2, 119.4, 111.10a): the life is part of the cost, paid with the mana and
/// the tap.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}, {T}").and(life(1)), tokens::create_a_treasure, "{1}, {T}, Pay 1 life: Create a Treasure token.")],
    ..oracle::TREASURE_DREDGER
};
