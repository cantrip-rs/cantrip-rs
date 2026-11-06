use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, statics::plus_pt_saturating, targets, triggers::ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP_HOST};
use cantrip_oracle as oracle;

/// Put the counter on the creature whose controller's upkeep caused the ability to trigger.
fn put_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject
        && is_live(g, creature)
    {
        g.add_counters(creature, CounterKind::MinusOne, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP_HOST,
        put_counter,
        "At the beginning of the upkeep of enchanted creature's controller, put a -1/-1 counter on that creature.",
    )],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt_saturating::<3, 3>, "Enchanted creature gets +3/+3.")],
    enchant: Some(targets::CREATURE),
    ..oracle::UNSTABLE_MUTATION
};
