use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::{self, THIS_ENTERS}};
use cantrip_oracle as oracle;

/// Prowess is `triggers::PROWESS` (CR 702.108a), listed first as the card's text lists it. "When this creature enters, surveil 2"
/// (CR 701.25).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::PROWESS,
        triggered(
            THIS_ENTERS,
            |g, item| effects::surveil(g, item, 2),
            "When this creature enters, surveil 2. (Look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)",
        ),
    ],
    ..oracle::LIGHTSHELL_DUO
};
