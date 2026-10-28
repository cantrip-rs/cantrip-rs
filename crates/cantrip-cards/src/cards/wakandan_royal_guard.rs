use cantrip_api::*;
use cantrip_mechanics::{targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this creature enters, put a +1/+1 counter on target creature. If that creature is another Hero, put two +1/+1 counters on
/// it instead." (CR 122.1, 608.2b, 608.2c): the target, judged as the ability resolves, is another Hero when it is not this creature
/// and has the creature type Hero, read through the words of this card's text (CR 612); the two replace the one, so it gets either
/// one or two, never three. Vigilance is a core keyword.
fn put_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let hero = g.words(item.object).subtype(Subtype::Hero);
    let another_hero = creature != item.object && g.chars(creature).has_subtype(hero);
    g.add_counters(creature, CounterKind::PlusOne, if another_hero { 2 } else { 1 });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, put_counters, "When this creature enters, put a +1/+1 counter on target creature. If that creature is another Hero, put two +1/+1 counters on it instead.")
        .targets(&[targets::CREATURE])],
    ..oracle::WAKANDAN_ROYAL_GUARD
};
