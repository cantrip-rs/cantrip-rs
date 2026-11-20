use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::Trample]),
    abilities: &[triggered(
        triggers::ENCHANTMENT_YOU_CONTROL_ENTERS,
        |g, item| effects::may_pay_to_return_this_to_hand(g, item, cost("{2}")),
        "Whenever an enchantment you control enters, you may pay {2}. If you do, return this card from your graveyard to your hand.",
    )
    // It returns this card from the graveyard, so it functions only there (CR 113.6m).
    .functions_in(IN_A_GRAVEYARD)],
    ..oracle::REDTOOTH_VANGUARD
};
