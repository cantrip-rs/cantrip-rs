use cantrip_api::*;
use cantrip_mechanics::{effects, flashback, targets};
use cantrip_oracle as oracle;

/// "Return target creature card with mana value 3 or less from your graveyard to the battlefield with an additional +1/+1 counter
/// on it"
pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::CREATURE_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD],
        effects::return_first_target_to_battlefield_with_counters::<{ CounterKind::PlusOne as usize }, 1>,
    ),
    ways: &[flashback::way(cost("{4}{W}"), "Flashback {4}{W}")],
    ..oracle::GENEROUS_REVIVAL
};
