use cantrip_api::*;
use cantrip_mechanics::{effects, sneak};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], |g, item| effects::draw(g, item, 2)),
    ways: &[sneak::way(mana("{U}"), "Sneak {U}")],
    ..oracle::DONATELLOS_TECHNIQUE
};
