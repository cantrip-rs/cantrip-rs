use cantrip_api::*;
use cantrip_mechanics::{effects, sneak};
use cantrip_oracle as oracle;

/// "Jennika's Technique deals 2 damage to each creature." Every creature on the battlefield as it resolves, all at once.
fn damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_creature(g, item, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], damage),
    ways: &[sneak::way(mana("{R}"), "Sneak {R}")],
    ..oracle::JENNIKAS_TECHNIQUE
};
