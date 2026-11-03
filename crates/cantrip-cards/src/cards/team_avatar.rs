use cantrip_api::*;
use cantrip_mechanics::{creatures_you_control, effects, targets, triggers::CREATURE_YOU_CONTROL_ATTACKS_ALONE};
use cantrip_oracle as oracle;

/// "It gets +X/+X until end of turn, where X is the number of creatures you control" (CR 506.5, 611.2a, 608.2h): the creature that
/// attacked alone, whichever creature of its controller's it is. X is worked out once, as the ability resolves (ruling 2025-10-02).
fn pump_by_creatures_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    let x = creatures_you_control(g, item.controller) as i16;
    cantrip_mechanics::pump(g, item.object, creature, x, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            CREATURE_YOU_CONTROL_ATTACKS_ALONE,
            pump_by_creatures_you_control,
            "Whenever a creature you control attacks alone, it gets +X/+X until end of turn, where X is the number of creatures you control.",
        ),
        ability(
            cost("{2}{W}").and(DISCARD_THIS),
            effects::damage_first_target_by_creatures_you_control,
            "{2}{W}, Discard this card: It deals damage equal to the number of creatures you control to target creature.",
        )
        .targets(&[targets::CREATURE])
        .from(ZoneKind::Hand),
    ],
    ..oracle::TEAM_AVATAR
};
