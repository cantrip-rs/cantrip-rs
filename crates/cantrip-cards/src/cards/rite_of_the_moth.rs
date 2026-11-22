use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::flashback;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD], effects::return_first_target_to_battlefield_with_counters::<{ CounterKind::Finality as usize }, 1>),
    ways: &[flashback::way(cost("{3}{W}{W}{B}"), "Flashback {3}{W}{W}{B}")],
    ..oracle::RITE_OF_THE_MOTH
};
