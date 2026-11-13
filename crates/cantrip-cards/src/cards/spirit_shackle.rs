use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// Put a -0/-2 counter on the creature that became tapped (CR 122.1a, 603.2).
fn put_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = item.subject.object().filter(|&id| targets::permanent(g, id)) {
        g.add_counters(creature, CounterKind::Plus0Minus2, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::ATTACHED_PERMANENT_BECOMES_TAPPED,
        put_counter,
        "Whenever enchanted creature becomes tapped, put a -0/-2 counter on it.",
    )],
    enchant: Some(targets::CREATURE),
    ..oracle::SPIRIT_SHACKLE
};
