use cantrip_api::*;
use cantrip_mechanics::combat::choose_attack_target;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Target creature you own": a creature on the battlefield whose owner is the ability's controller, whoever controls it (CR 108.3).
fn creature_you_own(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.obj(id).owner == asker.controller)
}

/// "Exile up to one target creature you own, then return it to the battlefield under your control tapped and attacking" (CR 400.7,
/// 508.4): it comes back as a new object under the ability's controller, tapped, attacking a player or planeswalker its controller
/// chooses, which need not be what The Neutrinos attack; it was never declared as an attacker, so nothing that watches attackers
/// being declared sees it. A token that is exiled ceases to exist and does not return (CR 111.7). With nothing it can attack, it
/// just returns tapped. Nothing happens if the creature has left the battlefield by now (CR 608.2b).
fn blink_tapped_and_attacking(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let token = g.obj(creature).token;
    let exiled = g.exile(creature);
    if token || !g.obj(exiled).zone.is(ZoneKind::Exile) {
        return Ok(());
    }
    let you = item.controller;
    let defender = choose_attack_target(g, you, item.object)?.unwrap_or(Target::None);
    g.put_onto_battlefield_attacking(exiled, you, defender, &[])?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::ANOTHER_CREATURE_YOU_CONTROL_ENTERS,
            effects::this_gets::<1, 0>,
            "Alliance \u{2014} Whenever another creature you control enters, The Neutrinos get +1/+0 until end of turn.",
        ),
        triggered(
            triggers::THIS_ATTACKS,
            blink_tapped_and_attacking,
            "Whenever The Neutrinos attack, exile up to one target creature you own, then return it to the battlefield under your control tapped and attacking.",
        )
        .targets(&[up_to_one(creature_you_own)]),
    ],
    ..oracle::THE_NEUTRINOS
};
