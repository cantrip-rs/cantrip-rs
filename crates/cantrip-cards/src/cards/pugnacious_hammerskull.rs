use cantrip_api::*;
use cantrip_mechanics::statics::you_control_another;
use cantrip_mechanics::{lands, triggers};
use cantrip_oracle as oracle;

/// "While you don't control another Dinosaur" (CR 603.2, 205.3m): asked as the creature attacks, never as the trigger resolves, so
/// a Dinosaur that comes or goes after it was declared as an attacker changes nothing (the ruling). An opponent's Dinosaur does not count.
fn you_dont_control_another_dinosaur(q: &dyn Query, asker: &Asker) -> bool {
    !you_control_another::<{ lands::sub(Subtype::Dinosaur) }>(q, asker)
}

/// "Put a stun counter on it" (CR 122.1d): on this creature only if it is still on the battlefield.
fn stun_itself(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Stun, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        stun_itself,
        "Whenever this creature attacks while you don't control another Dinosaur, put a stun counter on it. (If a permanent with a stun counter would become untapped, remove one from it instead.)",
    )
    .only_if(you_dont_control_another_dinosaur)],
    ..oracle::PUGNACIOUS_HAMMERSKULL
};
