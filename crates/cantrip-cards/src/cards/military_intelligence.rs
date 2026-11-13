use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ATTACKERS_DECLARED, triggers::you_attack_with_at_least::<2>),
        |g, item| effects::draw(g, item, 1),
        "Whenever you attack with two or more creatures, draw a card.",
    )],
    ..oracle::MILITARY_INTELLIGENCE
};
