use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "When Thorin enters, attach any number of target Equipment you control to target creature you control. When one or more
/// Equipment become attached to that creature this way, that creature deals damage equal to its power to up to one target
/// creature."
///
/// The slots are in printed order, the Equipment first (any number, CR 115.1d) and the creature after them; each target
/// records its slot (`StackItem::spec_targets`). A creature that is no longer a legal target (CR 608.2b) has nothing for the Equipment to be attached
/// to; each Equipment that is not a legal target stays where it is. An Equipment already attached to the creature does not
/// move and does not "become attached" (CR 701.3b), and one that can't be attached to it stays put. The second sentence is
/// a reflexive triggered ability (CR 603.12) that triggers only if at least one Equipment really became attached, carrying
/// the creature as its subject.
fn attach_equipment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = g.state().spec_targets(item, 1).next().and_then(|slot| targets::object(g, item, slot)) else { return Ok(()) };
    let mut attached = false;
    for slot in g.state().spec_targets(item, 0) {
        let Some(equipment) = targets::object(g, item, slot) else { continue };
        attached |= g.attach(equipment, creature);
    }
    if attached {
        let me = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(me, 0), Target::Object(creature));
    }
    Ok(())
}

/// "That creature deals damage equal to its power to up to one target creature": the damage is dealt by the creature
/// (CR 120.3), so its keywords count, and its power is read from last known information if it has left (CR 608.2h). With
/// no target chosen, or the target gone (CR 608.2b), there is no damage.
fn creature_deals_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    if g.target_legal(item, 0) {
        let power = g.chars(creature).power.unwrap_or(0).max(0) as u16;
        g.damage(creature, item.target(0), i64::from(power));
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        attach_equipment,
        "When Thorin enters, attach any number of target Equipment you control to target creature you control. When one or more Equipment become attached to that creature this way, that creature deals damage equal to its power to up to one target creature.",
    )
    .targets(&[
        up_to(TargetSpec::ANY_NUMBER, targets::EQUIPMENT_YOU_CONTROL.filter).labelled("Equipment you control to attach (any number)"),
        targets::CREATURE_YOU_CONTROL.labelled("Creature you control to attach the Equipment to"),
    ])],
    delayed: &[delayed_trigger(
        creature_deals_damage,
        "When one or more Equipment become attached to that creature this way, that creature deals damage equal to its power to up to one target creature.",
    )
    .targets(&[targets::CREATURE.optional().labelled("Creature to be dealt damage equal to the creature's power")])],
    ..oracle::THORIN_MOUNTAIN_KING
};
