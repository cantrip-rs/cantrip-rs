use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // Eerie is an ability word (CR 207.2c): the ability is an ordinary triggered ability. "This creature gains lifelink until end of
    // turn" (CR 702.15, 611.2a) is the source itself, nothing once it has left the battlefield (CR 400.7).
    abilities: &[triggered(
        triggers::EERIE,
        |g, item| effects::this_gains_until_end_of_turn(g, item, Keyword::Lifelink),
        "Eerie \u{2014} Whenever an enchantment you control enters and whenever you fully unlock a Room, this creature gains lifelink until end of turn.",
    )],
    ..oracle::CULT_HEALER
};
