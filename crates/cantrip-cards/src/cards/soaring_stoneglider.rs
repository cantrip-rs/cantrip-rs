use cantrip_api::*;
use cantrip_oracle as oracle;

/// Flying and vigilance are core keywords read from its characteristics.
pub static CARD: CardDef = CardDef {
    // "As an additional cost to cast this spell, exile two cards from your graveyard or pay {1}{W}" (CR 601.2b, 118.8).
    additional_choice: &[
        ExtraCost { text: "Exile two cards from your graveyard", cost: exile_from_graveyard(2) },
        ExtraCost { text: "Pay {1}{W}", cost: Cost::mana(mana("{1}{W}")) },
    ],
    ..oracle::SOARING_STONEGLIDER
};
