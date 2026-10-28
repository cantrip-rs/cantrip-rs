use cantrip_api::*;
use cantrip_mechanics::{pump, spent, targets, triggers};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Tap target creature an opponent controls and put a stun counter on it" (CR 122.1d): a creature already tapped still gets
/// the counter.
fn tap_and_stun(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.tap(creature);
    g.add_counters(creature, CounterKind::Stun, 1);
    Ok(())
}

/// Opus (an ability word, CR 207.2c): "Whenever you cast an instant or sorcery spell, this creature gets +1/+1 until end of turn.
/// If five or more mana was spent to cast that spell, this creature gets +2/+2 until end of turn instead." "Instead"
/// replaces the smaller bonus with the larger (CR 614.1a): one bonus, never both. The spell's mana spent (CR 601.2h) is read
/// from it, even if it has been countered or has left the stack (CR 608.2h).
fn opus(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let bonus = if spent::five_or_more_mana_spent_on_the_spell(g, item) { 2 } else { 1 };
    pump(g, item.object, item.object, bonus, bonus);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tap_and_stun, "When this creature enters, tap target creature an opponent controls and put a stun counter on it.")
            .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        triggered(
            triggers::INSTANT_OR_SORCERY_YOU_CAST,
            opus,
            "Opus \u{2014} Whenever you cast an instant or sorcery spell, this creature gets +1/+1 until end of turn. If five or more mana was spent to cast that spell, this creature gets +2/+2 until end of turn instead.",
        ),
    ],
    ..oracle::DELUGE_VIRTUOSO
};
