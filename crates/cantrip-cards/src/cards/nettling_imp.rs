use cantrip_api::*;
use cantrip_mechanics::{delayed, targets};
use cantrip_oracle as oracle;

/// A non-Wall creature the active player has controlled since the turn began (CR 302.6).
fn eligible(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    targets::permanent(q, id) && q.chars(id).is(CardType::Creature)
        && !q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Wall))
        && q.controller(id) == q.state().turn.active
        && q.obj(id).controlled_since < q.state().turn.number
}

/// The chosen creature must attack this turn if able (CR 508.1d).
fn must_attack(_: &dyn Query, _: &Asker, about: Target, id: ObjectId) -> bool {
    about.object() == Some(id)
}

/// Destroy it at the next end step if it did not attack that turn (CR 603.7, 701.8).
fn destroy_if_not_attacked(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject
        && !g.history().attacked_this_turn(creature)
    {
        g.destroy(creature);
    }
    Ok(())
}

fn nettle(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_lasting_rule(item, 0, Target::Object(creature), Duration::EndOfTurn);
    g.end_sentence();
    delayed::at_next_end_step(g, item, 0, Target::Object(creature));
    Ok(())
}

/// Only on an opponent's turn, before the first declare attackers step (2013-09-20 ruling).
fn before_attackers(q: &dyn Query, asker: &Asker) -> bool {
    q.state().opponents(asker.controller).any(|p| p == q.state().turn.active)
        && q.state().turn.step() < Step::DeclareAttackers
        && !q.history().this_turn().iter().any(|r| matches!(r.event, Event::StepBegan { step: Step::DeclareAttackers }))
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, nettle, "{T}: Choose target non-Wall creature the active player has controlled continuously since the beginning of the turn. That creature attacks this turn if able. Destroy it at the beginning of the next end step if it didn't attack this turn. Activate only during an opponent's turn, before attackers are declared.")
        .targets(&[target(eligible)]).condition(before_attackers)],
    delayed: &[delayed_trigger(destroy_if_not_attacked, "Destroy it at the beginning of the next end step if it didn't attack this turn.")],
    lasting_rules: &[LastingRule { text: "That creature attacks this turn if able.", rule: LastingRuleKind::MustAttack, applies: must_attack }],
    ..oracle::NETTLING_IMP
};
