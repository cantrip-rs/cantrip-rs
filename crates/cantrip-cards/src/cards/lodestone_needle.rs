use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{craft, effects, explore, targets};
use cantrip_oracle as oracle;

/// "Target creature you control explores." (CR 701.44) Its one target illegal, the ability does not resolve at all (CR
/// 608.2b), so the target is the creature.
fn target_explores(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.target(0) else { return Ok(()) };
    explore(g, creature)
}

pub static BACK: CardDef = CardDef {
    abilities: &[ability(cost("{1}, {T}"), target_explores, "{1}, {T}: Target creature you control explores. Activate only as a sorcery.")
        .targets(&[targets::CREATURE_YOU_CONTROL])
        .sorcery_speed()],
    ..oracle::GUIDESTONE_COMPASS
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::tap_and_stun::<2>, "When this artifact enters, tap up to one target artifact or creature and put two stun counters on it.")
            .targets(&[up_to_one(targets::artifact_or_creature_filter)]),
        craft::ability(mana("{2}{U}"), &craft::ARTIFACT, "Craft with artifact {2}{U}"),
    ],
    back: Some(&BACK),
    ..oracle::LODESTONE_NEEDLE
};
