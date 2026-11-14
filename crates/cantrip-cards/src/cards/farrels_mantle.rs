use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("DealDamage", "have the attacking creature deal damage to the target?", "If you do, it assigns no combat damage this turn.", "Deal damage", "Decline").no_default();

/// "Whenever enchanted creature attacks and isn't blocked, its controller may have it deal damage equal to its power plus 2 to another target creature. If that player does, the attacking creature assigns no combat damage this turn." (CR 115.1).
fn controller_have_deal_damage_equal_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    targets::creature_filter(q, asker, t) && t != q.event_subject()
}

/// "Whenever enchanted creature attacks and isn't blocked" (CR 603.2).
fn when_enchanted_creature_attacks_isnt_blocked(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Unblocked { attacker, .. } = event else { return None };
    let id = *attacker;
    let subject = Target::Object(id);
    if Some(asker.source).and_then(|o: ObjectId| q.attached_to(o)) == Some(id) {
        return Fired::about(subject);
    }
    None
}

/// "Whenever enchanted creature attacks and isn't blocked, its controller may have it deal damage equal to its power plus 2 to another target creature. If that player does, the attacking creature assigns no combat damage this turn." (CR 608.2h, 611.2a, 118.12).
fn controller_have_deal_damage_equal(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(attacker) = item.subject.object() else { return Ok(()) };
    let controller = g.controller(attacker);
    if effects::ask_may(g, controller, item.object, Purpose::Ask(&ASK))? {
        let amount = (i64::from(g.chars(attacker).power.unwrap_or(0)) + 2).max(0) as u32;
        g.damage(attacker, g.state().target(item, 0), amount);
        g.end_sentence();
        g.add_effect(item.object, attacker, EffectKind::NoCombatDamageAssignment, Duration::EndOfTurn);
    }

    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                when_asked(
                    EventMask::UNBLOCKED,
                    when_enchanted_creature_attacks_isnt_blocked,
                ),
                controller_have_deal_damage_equal,
                "Whenever enchanted creature attacks and isn't blocked, its controller may have it deal damage equal to its power plus 2 to another target creature. If that player does, the attacking creature assigns no combat damage this turn.",
            )
            .targets(&[target(controller_have_deal_damage_equal_target_0)]),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::FARRELS_MANTLE
};
