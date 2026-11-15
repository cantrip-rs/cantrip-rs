use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever this creature deals damage to an opponent, draw a card" (CR 603.2, 120.2, 121.1): one trigger for each source of
/// damage event, so combat damage to an opponent and, say, a damage spell it was given each draw once.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::DAMAGE, triggers::this_deals_damage_to_opponent),
        |g, item| effects::draw(g, item, 1),
        "Whenever this creature deals damage to an opponent, draw a card.",
    )],
    ..oracle::THIEVING_OTTER
};
