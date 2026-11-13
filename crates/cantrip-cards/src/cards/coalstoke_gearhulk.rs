use cantrip_api::*;
use cantrip_mechanics::{delayed, targets, triggers};
use cantrip_oracle as oracle;

/// "Put target creature card with mana value 4 or less from a graveyard onto the battlefield under your control with a finality
/// counter on it. That creature gains menace, deathtouch, and haste. At the beginning of your next end step, exile that
/// creature" (CR 122.6, 614.1c, 603.7): the counter is on it as it enters. The keywords last as long as the object does
/// (CR 611.2a, 400.7), and the delayed ability is about exactly the object that arrived (CR 603.7c). It is set up at once, so
/// it is this turn's end step when the creature arrives before it.
fn reanimate_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let creature = g.put_onto_battlefield_with_counters(card, item.controller, false, &[(CounterKind::Finality, 1)])?;
    let keywords = Keywords::of(&[Keyword::Menace, Keyword::Deathtouch, Keyword::Haste]);
    g.add_effect(item.object, creature, EffectKind::AddKeywords(keywords), Duration::Indefinite);
    delayed::at_your_next_end_step_about(g, item, 0, &[creature]);
    Ok(())
}

/// "Target creature card with mana value 4 or less from a graveyard" (CR 115.1, 202.3): a creature card in any graveyard whose mana
/// value there (X is 0, CR 202.3e) is at most 4.
fn creature_card_with_mana_value_4_or_less_in_a_graveyard(q: &dyn Query, _: &Asker, t: Target) -> bool {
    targets::card_in_graveyard(q, t, &[CardType::Creature]) && matches!(t, Target::Object(id) if q.mana_value(id) <= 4)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        reanimate_it,
        "When this creature enters, put target creature card with mana value 4 or less from a graveyard onto the battlefield under your control with a finality counter on it. That creature gains menace, deathtouch, and haste. At the beginning of your next end step, exile that creature.",
    )
    .targets(&[target(creature_card_with_mana_value_4_or_less_in_a_graveyard)])],
    delayed: &[delayed_trigger(delayed::exile_subject, "At the beginning of your next end step, exile that creature.")],
    ..oracle::COALSTOKE_GEARHULK
};
