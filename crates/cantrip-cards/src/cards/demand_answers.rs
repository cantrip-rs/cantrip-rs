use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // Draw two cards.
    spell: spell(&[], |g, item| effects::draw(g, item, 2)),
    // As an additional cost to cast this spell, sacrifice an artifact or discard a card (CR 601.2b).
    additional_choice: &[
        ExtraCost { text: "Sacrifice an artifact", cost: Cost::sacrifice(1, targets::filters::artifact) },
        ExtraCost { text: "Discard a card", cost: discard_cards(1) },
    ],
    ..oracle::DEMAND_ANSWERS
};
