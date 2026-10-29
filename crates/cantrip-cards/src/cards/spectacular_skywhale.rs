use cantrip_api::*;
use cantrip_mechanics::{pump, spent, triggers};
use cantrip_oracle as oracle;

/// Opus (an ability word, CR 207.2c): "Whenever you cast an instant or sorcery spell, this creature gets +3/+0 until end of turn. If
/// five or more mana was spent to cast that spell, put three +1/+1 counters on this creature instead." The spell's mana spent (CR
/// 601.2h) is read from it even if it has since been countered or has left the stack (CR 608.2h); a spell cast without paying its
/// mana cost has had none spent. "Instead" replaces the pump with the counters (CR 614.1a, 608.2e): never both.
fn opus(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if spent::five_or_more_mana_spent_on_the_spell(g, item) {
        g.add_counters(item.object, CounterKind::PlusOne, 3);
    } else {
        pump(g, item.object, item.object, 3, 0);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::INSTANT_OR_SORCERY_YOU_CAST,
        opus,
        "Opus \u{2014} Whenever you cast an instant or sorcery spell, this creature gets +3/+0 until end of turn. If five or more mana was spent to cast that spell, put three +1/+1 counters on this creature instead.",
    )],
    ..oracle::SPECTACULAR_SKYWHALE
};
