use cantrip_api::*;
use cantrip_mechanics::{exile_target, targets};
use cantrip_oracle as oracle;

/// "Exile target creature with mana value 3 or less" (CR 701.13, 608.2b, 202.3): the target is judged again as the spell
/// resolves, so a creature that has grown past 3 in mana value meanwhile (a copy effect, a changed cost) is an illegal target
/// and nothing is exiled.
fn exile_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(targets::creature_with_mana_value_at_most::<3>)], exile_it),
    ..oracle::DEATH_IN_THE_FAMILY
};
