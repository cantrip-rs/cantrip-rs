use cantrip_api::*;
use cantrip_mechanics::triggers::creature_attacks_with_another;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when(EventMask::ATTACKERS_DECLARED, creature_attacks_with_another), |g, item| cantrip_mechanics::effects::draw(g, item, 1), "Whenever Sokka and at least one other creature attack, draw a card.")],
    ..oracle::SOKKA_LATERAL_STRATEGIST
};
