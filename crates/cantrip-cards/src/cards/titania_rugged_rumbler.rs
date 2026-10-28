use cantrip_api::*;
use cantrip_mechanics::triggers::ward_discard_or_pay;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "As an additional cost to cast this spell, discard a card or pay {2}" (CR 601.2b).
    additional_choice: &[
        ExtraCost { text: "Discard a card", cost: discard_cards(1) },
        ExtraCost { text: "Pay {2}", cost: Cost::mana(mana("{2}")) },
    ],
    abilities: &[ward_discard_or_pay::<2>("Ward—Discard a card or pay {2}.")],
    ..oracle::TITANIA_RUGGED_RUMBLER
};
