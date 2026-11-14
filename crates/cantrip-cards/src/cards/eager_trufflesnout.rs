use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "Whenever this creature deals combat damage to a player, create a Food token." (CR 510.2, 603.2, 111.10b)
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER,
        tokens::create_a_food_token,
        "Whenever this creature deals combat damage to a player, create a Food token.",
    )],
    ..oracle::EAGER_TRUFFLESNOUT
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
