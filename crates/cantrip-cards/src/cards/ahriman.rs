use cantrip_api::*;
use cantrip_mechanics::{effects, targets::filters};
use cantrip_oracle as oracle;

/// Flying and deathtouch are core keywords. "{3}, Sacrifice another creature or artifact: Draw a card" (CR 118.3, 701.21): the
/// sacrifice is a cost, so it is paid before the draw and the ability is lost to no response.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}").and(sacrifice(1, filters::another_creature_or_artifact)),
        |g, item| effects::draw(g, item, 1),
        "{3}, Sacrifice another creature or artifact: Draw a card.",
    )],
    ..oracle::AHRIMAN
};
