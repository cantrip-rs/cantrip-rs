use cantrip_api::*;
use cantrip_mechanics::{attach, connive, lands, targets, triggers};
use cantrip_oracle as oracle;

/// "Attach up to one target Equipment you control to target creature you control." (CR 701.3a, 608.2b): both targets are chosen as
/// the ability goes on the stack, the Equipment optionally. With either gone or no longer legal by the time it resolves there
/// is nothing to attach, or nothing to attach it to, and the other is left alone.
fn attach_equipment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let (Some(equipment), Some(creature)) = (targets::object(g, item, 0), targets::object(g, item, 1)) {
        g.attach(equipment, creature);
    }
    Ok(())
}

/// "Whenever an equipped creature you control attacks" (CR 508.3a, 603.2c, 301.5a): declaring attackers names each attacker in
/// its own event, so the ability triggers once for each attacking creature of its controller's that has an Equipment attached to
/// it as it is declared, and is about that attacker. A creature put onto the battlefield attacking was never declared.
fn equipped_creature_you_control_attacks(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if q.controller(*attacker) == me.controller && attach::is_equipped(q, *attacker) => {
            Fired::about(Target::Object(*attacker))
        }
        _ => None,
    }
}

/// "It connives." (CR 701.50): the attacking creature, which connives even if it has left the battlefield.
fn it_connives(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    connive(g, creature, item.object)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Villain) }>),
            attach_equipment,
            "Whenever another Villain you control enters, attach up to one target Equipment you control to target creature you control.",
        )
        .targets(&[up_to_one(targets::EQUIPMENT_YOU_CONTROL.filter), targets::CREATURE_YOU_CONTROL]),
        triggered(
            when_asked(EventMask::ATTACKS, equipped_creature_you_control_attacks),
            it_connives,
            "Whenever an equipped creature you control attacks, it connives.",
        ),
    ],
    ..oracle::SWORDSMAN_SHARP_SCOUNDREL
};
