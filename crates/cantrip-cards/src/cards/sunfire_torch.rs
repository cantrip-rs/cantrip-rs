use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{grant_delayed_ability, plus_pt};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks" (CR 508.1, 603.2), of the ability a Sunfire Torch gave the creature (CR 613.1f): the
/// creature with it, declared as an attacker. "Sunfire Torch" in its text is the Torch that gave this instance of it, so each
/// of two Torches on one creature gives an ability naming itself: the Torch is the ability's giver as the layers have it
/// (`Query::ability_giver`), carried as the trigger's subject, so it is the same Torch as the ability resolves.
fn this_creature_attacks(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::Attacks { attacker, .. } = event else { return None };
    if *attacker != asker.source {
        return None;
    }
    Fired::about(Target::Object(q.ability_giver(asker)?))
}

/// "You may sacrifice Sunfire Torch. When you do, this creature deals 2 damage to any target." (CR 701.21a, 603.12): offered
/// only while the creature's controller can sacrifice the Torch (CR 608.2d, 101.2: not one another player controls, nor one that
/// has left); the reflexive ability, with its target chosen as it goes on the stack, exists only if it was sacrificed.
fn may_sacrifice_the_torch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let Some(torch) = item.subject.object().filter(|&torch| g.can_sacrifice(you, torch)) else { return Ok(()) };
    if effects::ask_may(g, you, item.object, Purpose::Sacrifice)? && !g.sacrifice(you, &[torch]).is_empty() {
        g.reflexive_trigger(item.object, you, AbilityRef::delayed(item.ability.card, 1), Target::None);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[equip(mana("{1}"), "Equip {1}")],
    delayed: &[
        triggered(
            when_asked(EventMask::ATTACKS, this_creature_attacks),
            may_sacrifice_the_torch,
            "Whenever this creature attacks, you may sacrifice Sunfire Torch. When you do, this creature deals 2 damage to any target.",
        ),
        delayed_trigger(effects::damage_first_target::<2>, "When you do, this creature deals 2 damage to any target.").targets(&[targets::ANY]),
    ],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 0>, "Equipped creature gets +1/+0."),
        static_ability(
            Layer::Ability,
            attached_object,
            grant_delayed_ability,
            "Equipped creature has \"Whenever this creature attacks, you may sacrifice Sunfire Torch. When you do, this creature deals 2 damage to any target.\"",
        ),
    ],
    ..oracle::SUNFIRE_TORCH
};
