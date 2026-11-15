use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Counter target spell unless its controller pays {2}. If you control a Bird, counter that spell unless its controller pays
/// {4} instead." (CR 118.12, 608.2c): the Bird is looked for as the spell resolves, and "Bird" is a word of this card's text
/// (CR 612), so the amount is chosen when the spell resolves, then the spell's controller is asked once.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let bird = g.words(item.object).subtype(Subtype::Bird);
    let price = if g.permanents_of(item.controller).into_iter().any(|id| g.chars(id).has_subtype(bird)) { 4 } else { 2 };
    cantrip_mechanics::counter_unless_pays(g, item, 0, generic(price))?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::SPELL], effect),
    ..oracle::DAZZLING_DENIAL
};
