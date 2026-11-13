use cantrip_api::*;
use cantrip_mechanics::statics::{have_cant_be_blocked, itself_with_threshold};
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this creature enters, surveil 3" (CR 701.25). Threshold (CR 207.2c, 604.1, 509.1b): "can't be blocked as long as there are seven or more
/// cards in your graveyard", asked again whenever characteristics are computed.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        |g, item| effects::surveil(g, item, 3),
        "When this creature enters, surveil 3. (Look at the top three cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)",
    )],
    statics: &[static_ability(
        Layer::Ability,
        itself_with_threshold,
        have_cant_be_blocked,
        "Threshold \u{2014} This creature can't be blocked as long as there are seven or more cards in your graveyard.",
    )],
    ..oracle::CEPHALID_INKMAGE
};
