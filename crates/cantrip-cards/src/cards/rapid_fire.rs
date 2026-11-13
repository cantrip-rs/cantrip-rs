use cantrip_api::*;
use cantrip_mechanics::{effects, rampage, targets};
use cantrip_oracle as oracle;

/// "Cast this spell only before blockers are declared." (CR 601.3).
fn cast_spell_only_before_blockers_if(q: &dyn Query, _: &Asker) -> bool {
    let events = q.history().this_turn();
    (q.state().turn.step() as u8) < (Step::DeclareBlockers as u8)
        && !events.iter().any(|r| matches!(r.event, Event::StepBegan { step: Step::DeclareBlockers | Step::EndOfCombat | Step::PostcombatMain | Step::End | Step::Cleanup }))
        && events.iter().filter(|r| matches!(r.event, Event::StepBegan { step: Step::BeginCombat })).take(2).count() < 2
}

/// "Cast this spell only before blockers are declared. Target creature gains first strike until end of turn. If it doesn't have rampage, that creature gains rampage 2 until end of turn. (Whenever the creature becomes blocked, it gets +2/+2 until end of turn for each creature blocking it beyond the first.)" (CR 608.2, 611.2a).
fn cast_spell_only_before_blockers(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_until_end_of_turn(g, item.object, creature, Keyword::FirstStrike);
    g.end_sentence();
    let has_rampage = g.chars(creature).abilities().any(|a| {
        g.triggered_ability(a).is_some_and(|ability| ability.keyword_family == Some(KeywordFamily::Rampage))
    });
    if !has_rampage {
        g.add_effect(item.object, creature, EffectKind::AddAbility(AbilityRef::delayed(item.ability.card, 0)), Duration::EndOfTurn);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], cast_spell_only_before_blockers),
    delayed: &[rampage::trigger::<2>("Rampage")],
    cast_only_if: Some(cast_spell_only_before_blockers_if),
    ..oracle::RAPID_FIRE
};
