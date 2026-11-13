use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::other_creatures_get::<-2, -2>, "When Harvester of Misery enters, other creatures get -2/-2 until end of turn."),
        ability(cost("{1}{B}").and(DISCARD_THIS), effects::target_creature_gets::<-2, -2>, "{1}{B}, Discard Harvester of Misery: Target creature gets -2/-2 until end of turn.")
            .targets(&[targets::CREATURE])
            .from(ZoneKind::Hand),
    ],
    ..oracle::HARVESTER_OF_MISERY
};
