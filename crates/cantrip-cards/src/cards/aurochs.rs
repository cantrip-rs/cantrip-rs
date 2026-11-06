use cantrip_api::*;
use cantrip_mechanics::{pump, triggers};
use cantrip_oracle as oracle;

/// Count the other attacking Aurochs as the ability resolves (CR 608.2h, 611.2c).
fn grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let subtype = g.words(item.object).subtype(Subtype::Aurochs);
    let count = g.state().attackers.iter()
        .filter(|a| a.id != item.object && g.chars(a.id).has_subtype(subtype))
        .count() as i32;
    pump(g, item.object, item.object, count, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        grow,
        "Whenever this creature attacks, it gets +1/+0 until end of turn for each other attacking Aurochs.",
    )],
    ..oracle::AUROCHS
};
