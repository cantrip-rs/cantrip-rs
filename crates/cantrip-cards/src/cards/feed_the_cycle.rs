use cantrip_api::*;
use cantrip_mechanics::{effects, forage, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_OR_PLANESWALKER], effects::destroy_first_target),
    // "As an additional cost to cast this spell, forage or pay {B}" (CR 601.2b, 701.61a).
    additional_choice: &[forage::EXILE_THREE_CARDS, forage::SACRIFICE_A_FOOD, ExtraCost { text: "Pay {B}", cost: Cost::mana(mana("{B}")) }],
    ..oracle::FEED_THE_CYCLE
};
