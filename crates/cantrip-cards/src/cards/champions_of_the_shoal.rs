use cantrip_api::*;
use cantrip_mechanics::triggers::{this_becomes_tapped, this_enters, THIS_LEAVES};
use cantrip_mechanics::{behold, effects, targets};
use cantrip_oracle as oracle;

/// "Whenever this creature enters or becomes tapped" (CR 603.6a, 603.2): either event.
fn this_enters_or_becomes_tapped(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    this_enters(q, source, event).or_else(|| this_becomes_tapped(q, source, event))
}

/// "Tap up to one target creature and put a stun counter on it" (CR 122.1d): a creature already tapped still gets the counter.
fn tap_and_stun(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.tap(creature);
    g.add_counters(creature, CounterKind::Stun, 1);
    Ok(())
}

/// "As an additional cost to cast this spell, behold a Merfolk and exile it" (CR 601.2b, 701.4a), exiled with the creature
/// the spell becomes (CR 607.2q), which returns it when it leaves the battlefield. A spell that is countered leaves it exiled.
pub static CARD: CardDef = CardDef {
    additional_cost: Some(behold::cost_and_exile(behold::merfolk, "Merfolk")),
    abilities: &[
        triggered(
            when(EventMask(EventMask::ZONE_CHANGE.0 | EventMask::TAPPED.0), this_enters_or_becomes_tapped),
            tap_and_stun,
            "Whenever this creature enters or becomes tapped, tap up to one target creature and put a stun counter on it.",
        )
        .targets(&[up_to_one(targets::CREATURE.filter)]),
        triggered(THIS_LEAVES, effects::return_exiled_to_hand, "When this creature leaves the battlefield, return the exiled card to its owner's hand."),
    ],
    ..oracle::CHAMPIONS_OF_THE_SHOAL
};
