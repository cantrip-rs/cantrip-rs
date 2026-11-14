use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::YOU_ATTACK;
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&tokens::GREMLIN];

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(YOU_ATTACK, tokens::create_a_gremlin, "Whenever you attack, create a 1/1 red Gremlin creature token.")],
    ..oracle::RAZORKIN_HORDECALLER
};
