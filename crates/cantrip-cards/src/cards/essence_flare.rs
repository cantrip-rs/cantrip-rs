use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, statics::plus_pt_saturating, targets, triggers::ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP_HOST};
use cantrip_oracle as oracle;

/// The trigger identifies that creature at the beginning of upkeep (CR 608.2k).
fn put_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = item.subject.object() {
        g.add_counters(creature, CounterKind::Plus0Minus1, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP_HOST,
        put_counter,
        "At the beginning of the upkeep of enchanted creature's controller, put a -0/-1 counter on that creature.",
    )],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt_saturating::<2, 0>, "Enchanted creature gets +2/+0.")],
    enchant: Some(targets::CREATURE),
    ..oracle::ESSENCE_FLARE
};
