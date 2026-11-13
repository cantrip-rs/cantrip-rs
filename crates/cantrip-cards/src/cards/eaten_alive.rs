use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "As an additional cost to cast this spell, sacrifice a creature or pay {3}{B}" (CR 601.2b, 118.8, 701.21a): one of the two,
/// chosen as it is cast. "Exile target creature or planeswalker" (CR 701.13a): nothing if it is no longer a legal target.
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_OR_PLANESWALKER], effects::exile_first_target),
    additional_choice: &[
        ExtraCost { text: "Sacrifice a creature", cost: Cost::sacrifice(1, targets::filters::creature) },
        ExtraCost { text: "Pay {3}{B}", cost: Cost { mana: mana("{3}{B}"), ..Cost::NONE } },
    ],
    ..oracle::EATEN_ALIVE
};
