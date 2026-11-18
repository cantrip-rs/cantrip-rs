use cantrip_api::*;
use cantrip_mechanics::forage::{EXILE_THREE_CARDS, SACRIFICE_A_FOOD};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "As an additional cost to cast this spell, forage or pay {B}" (CR 601.2b, 701.61a, 118.3): forage is its two ways, exiling three
/// cards from your graveyard or sacrificing a Food, so the choice is those two or the mana. "Destroy target creature or
/// planeswalker" (CR 701.8a).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_OR_PLANESWALKER], effects::destroy_first_target),
    additional_choice: &[EXILE_THREE_CARDS, SACRIFICE_A_FOOD, ExtraCost { text: "Pay {B}", cost: Cost::mana(mana("{B}")) }],
    ..oracle::FEED_THE_CYCLE
};
