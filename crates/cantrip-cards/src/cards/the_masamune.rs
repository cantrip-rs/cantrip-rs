use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{creature_dying, grant_delayed_ability, have_first_strike};
use cantrip_oracle as oracle;

/// "As long as equipped creature is attacking" (CR 506.3, 613.1f) as `StaticDef::affects`: the creature this Equipment is attached
/// to, while it is declared as an attacker or put onto the battlefield attacking and not removed from combat since. Asked again on
/// every pass, so a creature that stops attacking has first strike no longer.
fn equipped_creature_that_is_attacking(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    attached_object(view, source, affected, so_far) && view.state().attackers.iter().any(|a| a.id == affected)
}

/// "... must be blocked if able" (CR 509.1c) as a `CardDef::must_be_blocked` entry: the requirement is on the equipped creature as an
/// attacker, and only while it attacks. An Equipment's own rule, not an ability the creature gains (it has no "has"), so the
/// creature losing its abilities leaves it in force (CR 613.1f).
fn equipped_attacker_must_be_blocked_if_able(q: &dyn Query, source: ObjectId, attacker: ObjectId) -> bool {
    q.obj(source).attached_to == Some(attacker) && q.is_attacking(attacker)
}

/// "If a creature dying causes a triggered ability of this creature or an emblem you own to trigger, that ability triggers an
/// additional time" (CR 603.2d), as a rule the equipped creature is given (`granted_additional_trigger`): "this creature" is the
/// creature that has it, so it is the equipped creature's own abilities and not another permanent's, and an emblem "you own" is
/// the emblem of the creature's controller, who is the one it says "you" for. What counts as the dying is the engine's
/// (`statics::creature_dying`): the equipped creature's own death too, which its dies ability looks back to.
fn its_own_and_its_owners_emblems_triggers_again(q: &dyn Query, asker: &Asker, event: &Event, triggering: &Triggering) -> bool {
    creature_dying(q, event, triggering)
        && if triggering.permanent {
            triggering.source == asker.source
        } else {
            let source = q.obj(triggering.source);
            source.zone == Zone::COMMAND && source.owner == asker.controller
        }
}

pub static CARD: CardDef = CardDef {
    abilities: &[equip(mana("{2}"), "Equip {2}")],
    statics: &[
        static_ability(
            Layer::Ability,
            equipped_creature_that_is_attacking,
            have_first_strike,
            "As long as equipped creature is attacking, it has first strike and must be blocked if able.",
        ),
        static_ability(
            Layer::Ability,
            attached_object,
            grant_delayed_ability,
            "Equipped creature has \"If a creature dying causes a triggered ability of this creature or an emblem you own to trigger, that ability triggers an additional time.\"",
        ),
    ],
    must_be_blocked: &[equipped_attacker_must_be_blocked_if_able],
    // The rule the equipped creature has (CR 613.1f): its own, so its controller is the "you" (CR 113.8).
    delayed: &[granted_additional_trigger(
        its_own_and_its_owners_emblems_triggers_again,
        "If a creature dying causes a triggered ability of this creature or an emblem you own to trigger, that ability triggers an additional time.",
    )],
    ..oracle::THE_MASAMUNE
};
