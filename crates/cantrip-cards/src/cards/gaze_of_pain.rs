use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Until end of turn" (CR 603.2).
fn watch_2_event(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::Unblocked { attacker, .. } = event else { return None };
    let id = *attacker;
    let subject = Target::Object(id);
    if q.chars(id).is(CardType::Creature) && asker.controller == q.controller(id) {
        return Fired::about(subject);
    }
    None
}

const CHOOSE_DAMAGE: Question = Question::new(
    "GazeOfPain", "have the attacking creature deal damage equal to its power to the target?",
    "If you do, it assigns no combat damage this turn.", "Deal damage", "Don't deal damage",
).no_default();

/// The attacker deals damage equal to its current or last-known power, then assigns no combat damage if chosen (CR 608.2h, 120.3).
fn watch_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let Some(attacker) = item.subject.object() else { return Ok(()) };
    if effects::ask_may(g, item.controller, attacker, Purpose::Ask(&CHOOSE_DAMAGE))? {
        let power = targets::power_or_last_known(g, attacker).max(0) as u32;
        g.damage(attacker, Target::Object(target), power);
        g.end_sentence();
        g.add_effect(item.object, attacker, EffectKind::NoCombatDamageAssignment, Duration::EndOfTurn);
    }
    Ok(())
}

/// "Until end of turn, whenever a creature you control attacks and isn't blocked, you may choose to have it deal damage equal to its power to a target creature. If you do, it assigns no combat damage this turn." (CR 608.2).
fn whenever_creature_control_attacks_isnt(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    g.delayed_trigger(
        DelayedWhen::ThisTurn,
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        Target::None,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], whenever_creature_control_attacks_isnt),
    delayed: &[
        triggered(
                when_asked(EventMask::UNBLOCKED, watch_2_event),
                watch_2,
                "Until end of turn, whenever a creature you control attacks and isn't blocked, you may choose to have it deal damage equal to its power to a target creature.",
            )
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::GAZE_OF_PAIN
};
