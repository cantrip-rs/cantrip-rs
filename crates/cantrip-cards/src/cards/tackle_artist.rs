use cantrip_api::*;
use cantrip_mechanics::{spent, triggers};
use cantrip_oracle as oracle;

/// Opus (an ability word, CR 207.2c): "Whenever you cast an instant or sorcery spell, put a +1/+1 counter on this creature. If five
/// or more mana was spent to cast that spell, put two +1/+1 counters on this creature instead." The spell's mana spent (CR 601.2h)
/// is read from it even if it has since been countered or has left the stack (CR 608.2h); a spell cast without paying its mana cost
/// has had none spent. If this creature has left the battlefield by the time the trigger resolves, it gets no counters (CR 400.7).
fn opus(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let counters = if spent::five_or_more_mana_spent_on_the_spell(g, item) { 2 } else { 1 };
    g.add_counters(item.object, CounterKind::PlusOne, counters);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::INSTANT_OR_SORCERY_YOU_CAST,
        opus,
        "Opus \u{2014} Whenever you cast an instant or sorcery spell, put a +1/+1 counter on this creature. If five or more mana was spent to cast that spell, put two +1/+1 counters on this creature instead.",
    )],
    ..oracle::TACKLE_ARTIST
};
