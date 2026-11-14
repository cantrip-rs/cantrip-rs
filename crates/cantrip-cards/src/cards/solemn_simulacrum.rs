use cantrip_api::*;
use cantrip_mechanics::lands::may_fetch_basic;
use cantrip_mechanics::triggers::{THIS_DIES, THIS_ENTERS};
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "When this creature enters, you may search your library for a basic land card, put that card onto the battlefield tapped, then
/// shuffle" is `lands::may_fetch_basic` (tapped); "When this creature dies, you may draw a card" is `effects::may_draw_a_card`.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            |g, item| may_fetch_basic(g, item.controller, item.object, true),
            "When this creature enters, you may search your library for a basic land card, put that card onto the battlefield tapped, then shuffle.",
        ),
        triggered(THIS_DIES, effects::may_draw_a_card, "When this creature dies, you may draw a card."),
    ],
    ..oracle::SOLEMN_SIMULACRUM
};
