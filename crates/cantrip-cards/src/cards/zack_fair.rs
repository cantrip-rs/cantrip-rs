use cantrip_api::*;
use cantrip_mechanics::{choose, effects, targets};
use cantrip_oracle as oracle;

/// The Equipment that was attached to `zack` as he left the battlefield (CR 701.3d, 704.5n): the attachments his leaving
/// ended, which the history keeps as `Unattached` records made just before his own `ZoneChange` (he is the only permanent the
/// sacrifice moves). One that left him on some earlier occasion has its `Unattached` record there, with the cost's payment
/// and other records between, and is not counted. Only those still on the battlefield as the same object (CR 400.7) and
/// still Equipment.
fn equipment_attached_to(g: &dyn Game, zack: ObjectId) -> Vec<ObjectId> {
    let records = g.history().all();
    let left = records.iter().rposition(|r| matches!(r.event, Event::ZoneChange { old, from, .. } if old == zack && from.is(ZoneKind::Battlefield))).unwrap_or(0);
    let mut equipment: Vec<ObjectId> = records[..left]
        .iter()
        .rev()
        .take_while(|r| matches!(r.event, Event::Unattached { from, .. } if from == zack))
        .filter_map(|r| match r.event {
            Event::Unattached { attachment, .. } if targets::permanent(g, attachment) && g.chars(attachment).has_subtype(Subtype::Equipment) => Some(attachment),
            _ => None,
        })
        .collect();
    equipment.reverse();
    equipment
}

/// "Target creature you control gains indestructible until end of turn. Put Zack Fair's counters on that creature and attach an
/// Equipment that was attached to Zack Fair to that creature." The target is chosen as the ability is put on the stack, so Zack
/// himself may be chosen; sacrificed as the cost he is gone as it resolves and the ability does nothing (CR 608.2b). All his
/// counters, of every kind, as he last had them (CR 608.2h, the rulings). One Equipment is attached; with more than one that
/// was attached to him, its controller chooses (CR 701.3a), and one that cannot be attached to that creature stays where it is
/// (CR 701.3b).
fn sacrifice_effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_indestructible_until_end_of_turn(g, item.object, creature);
    // Every kind together, one event (CR 608.2, 122.6).
    let puts: Vec<(ObjectId, CounterKind, i64)> = CounterKind::ALL.into_iter().map(|kind| (creature, kind, i64::from(g.obj(item.object).counters(kind)))).collect();
    g.add_counters_together(&puts);
    let equipment = equipment_attached_to(g, item.object);
    let chosen = match equipment.as_slice() {
        [] => None,
        [only] => Some(*only),
        _ => choose::card(g, item.controller, item.object, Purpose::Attach, equipment)?,
    };
    if let Some(equipment) = chosen {
        g.attach(equipment, creature);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(cantrip_mechanics::effects::enters_with_a_counter::<{ CounterKind::PlusOne as usize }>),
    abilities: &[ability(
        cost("{1}").and(SACRIFICE_THIS),
        sacrifice_effect,
        "{1}, Sacrifice Zack Fair: Target creature you control gains indestructible until end of turn. Put Zack Fair's counters on that creature and attach an Equipment that was attached to Zack Fair to that creature.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::ZACK_FAIR
};
