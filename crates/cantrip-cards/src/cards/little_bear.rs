use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Untap another target creature you control. If that creature is a Bear, put a +1/+1 counter on it." (CR 701.26a, 608.2c): the
/// check is made as the ability resolves, after the untap, as the layers give the creature's types then; the type is read through
/// the words of this card's text (CR 612). Nothing at all if the target is no longer a legal target (CR 608.2b).
fn untap_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.untap(creature);
    g.end_sentence();
    let bear = g.words(item.object).subtype(Subtype::Bear);
    if g.chars(creature).has_subtype(bear) {
        g.add_counters(creature, CounterKind::PlusOne, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        untap_it,
        "When this creature enters, untap another target creature you control. If that creature is a Bear, put a +1/+1 counter on it.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ..oracle::LITTLE_BEAR
};
