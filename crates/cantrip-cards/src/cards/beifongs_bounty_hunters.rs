use cantrip_api::*;
use cantrip_mechanics::{earthbend, targets::power_or_last_known, triggers};
use cantrip_oracle as oracle;

/// "Whenever a nonland creature you control dies" (CR 700.4, 603.10a): `triggers::creature_you_control_dies`, looked
/// back on, narrowed to a creature that was no land as it last was on the battlefield (an earthbent land or a Mutavault dying
/// does not count).
/// Itself and a token count, an opponent's creature does not. The ability is about the creature that died, whose power it reads.
fn nonland_creature_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    triggers::creature_you_control_dies(q, me, event)?;
    let Event::ZoneChange { old, .. } = event else { return None };
    if q.chars(*old).is(CardType::Land) {
        return None;
    }
    Fired::about(Target::Object(*old))
}

/// "Earthbend X, where X is that creature's power" (CR 701.66a, 608.2h): the power the creature that died last had on the
/// battlefield, counters and pumps included, at least 0, for the shared earthbend of the land targeted as the trigger went on the stack.
fn earthbend_its_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = item.subject.object() else { return Ok(()) };
    let power = power_or_last_known(g, creature).max(0);
    earthbend::earthbend(g, item, power)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, nonland_creature_you_control_dies),
        earthbend_its_power,
        "Whenever a nonland creature you control dies, earthbend X, where X is that creature's power. (Target land you control becomes a 0/0 creature with haste that's still a land. Put X +1/+1 counters on it. When it dies or is exiled, return it to the battlefield tapped.)",
    )
    .targets(&[earthbend::TARGET])],
    delayed: &[earthbend::RETURN],
    ..oracle::BEIFONGS_BOUNTY_HUNTERS
};
