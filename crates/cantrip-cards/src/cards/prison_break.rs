use cantrip_api::*;
use cantrip_mechanics::{effects, mayhem, targets};
use cantrip_oracle as oracle;

/// "Return target creature card from your graveyard to the battlefield. It enters with an additional +1/+1 counter on it."
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD], effects::return_first_target_to_battlefield_with_counters::<{ CounterKind::PlusOne as usize }, 1>),
    ways: &[mayhem::way(mana("{3}{B}"), "Mayhem {3}{B}")],
    ..oracle::PRISON_BREAK
};
