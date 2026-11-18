use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::statics::{attacks_each_combat_if_able, itself};
use cantrip_oracle as oracle;

/// Haste is a core keyword read from its characteristics. "This creature attacks each combat if able" (CR 508.1d, layer 6) is the
/// shared static requirement; "{2}, Sacrifice this creature: Draw a card" (CR 602.2, 701.21) sacrifices it as a cost, so the
/// ability resolves without it, and it is the Clue ability printed on a Clue creature.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}").and(SACRIFICE_THIS), |g, item| effects::draw(g, item, 1), "{2}, Sacrifice this creature: Draw a card.")],
    statics: &[static_ability(Layer::Ability, itself, attacks_each_combat_if_able, "This creature attacks each combat if able.")],
    ..oracle::RED_HERRING
};
