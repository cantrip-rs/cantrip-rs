use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Tap up to one target creature. If an opponent controls it, put a stun counter on it" (CR 701.26a, 122.1d, 608.2c): the
/// counter is put only when the creature is controlled by an opponent of the ability's controller as the ability resolves;
/// nothing if the target is no longer legal (CR 608.2b).
fn tap_then_stun_if_an_opponent_controls_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.tap(creature);
    g.end_sentence();
    if g.controller(creature) != item.controller {
        g.add_counters(creature, CounterKind::Stun, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, tap_then_stun_if_an_opponent_controls_it, "When this creature enters, tap up to one target creature. If an opponent controls it, put a stun counter on it.")
        .targets(&[targets::CREATURE.optional()])],
    ..oracle::FEAR_OF_IMMOBILITY
};
