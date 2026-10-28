use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, effects, statics, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Tap enchanted creature and remove all counters from it." This trigger refers to the object attached when it resolves,
/// using last known information if needed (CR 303.4m, 608.2h).
fn tap_and_remove_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = g.obj(item.object).attached_to else { return Ok(()) };
    g.tap(creature);
    effects::remove_all_counters(g, creature);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(
        THIS_ENTERS,
        tap_and_remove_counters,
        "When this Aura enters, tap enchanted creature and remove all counters from it.",
    )],
    statics: &[loses_all_abilities(attached_object, "Enchanted creature loses all abilities.")],
    untap_bans: &[statics::attached_permanent_doesnt_untap],
    ..oracle::ENCHANTED_RIVERS_GRASP
};
