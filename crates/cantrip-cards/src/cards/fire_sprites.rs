use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Flying" (CR 604.1).
pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(cost("{G}").and(TAP), add(Mana::R), "{G}, {T}: Add {R}.")],
    ..oracle::FIRE_SPRITES
};
