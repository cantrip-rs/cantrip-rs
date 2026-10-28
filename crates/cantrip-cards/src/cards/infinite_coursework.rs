use cantrip_api::*;
use cantrip_mechanics::{attach::{attached_object, attached_to_it}, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Tap enchanted creature. It becomes unprepared." This trigger refers to the object attached when it resolves, using
/// last known information if needed (CR 303.4m, 608.2h).
fn tap_and_unprepare(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = g.obj(item.object).attached_to else { return Ok(()) };
    g.tap(creature);
    g.become_unprepared(creature);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(
        THIS_ENTERS,
        tap_and_unprepare,
        "When this Aura enters, tap enchanted creature. It becomes unprepared.",
    )],
    statics: &[
        loses_all_abilities(attached_object, "Enchanted creature loses all abilities."),
    ],
    // "Enchanted creature ... doesn't untap during its controller's untap step" (CR 502.3): a rule of the Aura's, not an
    // ability of the creature (CR 113.1a), so the creature losing all its abilities - by this Aura's own first half, at once -
    // leaves it in force (CR 613.1f).
    untap_bans: &[attached_to_it],
    ..oracle::INFINITE_COURSEWORK
};
