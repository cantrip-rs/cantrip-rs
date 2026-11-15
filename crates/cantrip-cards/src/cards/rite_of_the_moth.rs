use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::flashback;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD], effects::return_target_card_with_finality),
    ways: &[flashback::way(cost("{3}{W}{W}{B}"), "Flashback {3}{W}{W}{B}")],
    ..oracle::RITE_OF_THE_MOTH
};
