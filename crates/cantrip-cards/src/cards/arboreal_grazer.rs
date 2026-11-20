use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this creature enters, you may put a land card from your hand onto the battlefield tapped." (CR 608.2d): any land card of
/// the hand may be chosen, or none, with no land play used (`effects::may_put_a_land_from_hand_tapped`).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::may_put_a_land_from_hand_tapped,
        "When this creature enters, you may put a land card from your hand onto the battlefield tapped.",
    )],
    ..oracle::ARBOREAL_GRAZER
};
