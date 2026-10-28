//! Equipment and Auras (CR 301.5, 303.4): the equip ability and the "attached object" a static ability affects.

use cantrip_api::*;

use crate::targets;

/// "Attach this to target creature you control": the effect of equip, and of an attach ability at any speed.
pub fn attach_to_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.target_legal(item, 0)
        && let Target::Object(creature) = item.target(0)
    {
        g.attach(item.object, creature);
    }
    Ok(())
}

/// "Return this card from your graveyard to the battlefield attached to target [creature]" (CR 303.4i, 400.7): an Aura card put onto
/// the battlefield under its owner's control attached to the ability's first target, which is not chosen by the Aura's controller
/// (CR 303.4f) but must be a legal object for it to enchant, or it stays in the graveyard. It does nothing if the target is no
/// longer legal (CR 608.2b) or the card has left the graveyard (CR 400.7). Eagle's Rescue and Gryff's Boon print it.
pub fn return_this_to_the_battlefield_attached_to_first_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(host) = targets::object(g, item, 0)
        && is_live(g, item.object)
    {
        g.put_onto_battlefield_attached(item.object, item.controller, host)?;
    }
    Ok(())
}

/// Equip [cost] (CR 702.6a): "[cost]: Attach this to target creature you control. Activate only as a sorcery."
/// `text` is the ability's text, which a scenario's `Ability:` matches. Umezawa's Jitte and Lavaspur Boots print it.
pub const fn equip(mana: ManaCost, text: &'static str) -> AbilityDef {
    equip_cost(Cost::mana(mana), text)
}

/// Equip with any cost (CR 702.6a): "Equip\u{2014}Pay 3 life" (Dark Knight's Greatsword) and "Equip\u{2014}Sacrifice a creature"
/// (Dissection Tools). Chain `.once_each_turn()` where the card limits it.
pub const fn equip_cost(cost: Cost, text: &'static str) -> AbilityDef {
    ability(cost, attach_to_target, text).sorcery_speed().targets(&[targets::CREATURE_YOU_CONTROL]).equip()
}

/// Equip [quality] [cost] (CR 702.6a): "[cost]: Attach this to target [quality] creature you control. Equip only as a sorcery."
/// `creature` is the one target slot: the creature, its quality and "you control" included. Wrecking Ball Arm's "Equip legendary creature {3}" and
/// Mjölnir's "Equip worthy {1}" print it.
pub const fn equip_quality(mana: ManaCost, creature: &'static [TargetSpec], text: &'static str) -> AbilityDef {
    ability(Cost::mana(mana), attach_to_target, text).sorcery_speed().targets(creature).equip()
}

/// "Equip abilities you activate that target this creature cost {N} less to activate" (CR 601.2f, 602.2b): a static
/// ability of the creature, read with the targets announced for the equip ability (the total cost is determined after
/// them, CR 601.2c), taking off generic mana only (CR 118.7a: an equip cost of {G} is not reduced) and never below
/// {0}. Any equip ability, a variant with a restriction ("equip legendary creature") too; reconfigure is not one.
/// Dwarven Mauler and Helitrooper ({2}), Fervent Champion ({3}).
pub fn equip_targeting_this_costs_less<const N: u8>(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, targets: &[Target]) -> CostDelta {
    let Costed::Ability(_, ability, _) = costed else { return CostDelta::NONE };
    let equip = q.activated_ability(ability).is_some_and(|d| d.equip);
    if equip && player == q.controller(source) && targets.contains(&Target::Object(source)) {
        CostDelta::less(N)
    } else {
        CostDelta::NONE
    }
}

/// "You may attach that Equipment to target creature you control" (CR 701.3a), for a trigger that fired about the Equipment
/// (`Fired::about`) and targets a creature: nothing if the Equipment has left the battlefield (a new object, CR 400.7); a creature that is no longer a legal
/// target leaves the ability to do nothing (CR 608.2b); the choice is made as the ability resolves. Hammer of Nazahn and
/// Sigarda's Aid print it.
pub fn may_attach_that_equipment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(that) = item.subject else { return Ok(()) };
    if !targets::permanent(g, that) {
        return Ok(());
    }
    let Target::Object(creature) = item.target(0) else { return Ok(()) };
    if crate::effects::ask_may(g, item.controller, that, Purpose::Attach)? {
        g.attach(that, creature);
    }
    Ok(())
}

/// "You may attach this Equipment to it" (CR 701.3a) for an `equipment` and the `creature` the ability is about: the choice is
/// made as the ability resolves, and nothing is asked when the Equipment or the creature has left the battlefield (a new
/// object, CR 400.7) or the creature is no longer a creature, since it could not be attached (CR 301.5c, 701.3b). `purpose`
/// words the question. Doc Ock's Tentacles and Cori-Steel Cutter print it.
pub fn may_attach_to(g: &mut dyn Game, player: PlayerId, equipment: ObjectId, creature: ObjectId, purpose: Purpose) -> Result<(), Suspend> {
    if is_live(g, equipment)
        && g.is_on_battlefield(equipment)
        && is_live(g, creature)
        && g.is_on_battlefield(creature)
        && g.chars(creature).is(CardType::Creature)
        && crate::effects::ask_may(g, player, equipment, purpose)?
    {
        g.attach(equipment, creature);
    }
    Ok(())
}

/// "Whenever an Equipment you control enters" (CR 603.6a): the entering Equipment is what the trigger is about. Sigarda's Aid
/// prints it (Hammer of Nazahn's reads "this or another", so it has a matcher of its own).
pub fn equipment_you_control_enters(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if to.is(ZoneKind::Battlefield) && q.controller(*new) == q.controller(source) && q.chars(*new).has_subtype(Subtype::Equipment) =>
        {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

/// The Equipment `player` controls (CR 301.5), through the layers. The Lonely Mountain counts them, Battlefield
/// Improvisation chooses among them and "target Equipment you control" (`targets::EQUIPMENT_YOU_CONTROL`) is judged by it. Dwalin Weaponmaster too.
pub fn equipment_controlled(q: &dyn Query, player: PlayerId) -> Vec<ObjectId> {
    q.permanents_of(player).into_iter().filter(|&id| q.chars(id).has_subtype(Subtype::Equipment)).collect()
}

/// A static ability's `affects` for "equipped creature" and "enchanted creature" (CR 301.5, 303.4): exactly the
/// permanent the source is attached to.
pub fn attached_object(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    view.state().obj(source).attached_to == Some(affected)
}

/// Living weapon (CR 702.92a): create a 0/0 black Phyrexian Germ creature token, then attach this Equipment to it.
/// The Equipment attaches only if it is still on the battlefield and the token was made (CR 701.3b). Nettlecyst and
/// Kaldra Compleat print it.
pub fn living_weapon(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let germ = g.card_id(&crate::tokens::GERM);
    // The Germ is the keyword's (CR 702.92a), whatever the card's text says now.
    let made = g.create_tokens_worded(&[(germ, item.controller)], false, None, Words::NONE)?;
    if let Some(&token) = made.first()
        && is_live(g, token)
        && g.is_on_battlefield(token)
        && is_live(g, item.object)
        && g.is_on_battlefield(item.object)
    {
        g.attach(item.object, token);
    }
    Ok(())
}
