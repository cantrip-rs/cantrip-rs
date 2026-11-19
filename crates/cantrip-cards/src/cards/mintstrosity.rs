use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "When this creature dies, create a Food token." (CR 700.4, 603.10a, 111.10b): it looks back at the creature as it died, so the
/// token is made by its controller as it last existed on the battlefield.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_DIES, tokens::create_a_food_token, "When this creature dies, create a Food token.")],
    ..oracle::MINTSTROSITY
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
