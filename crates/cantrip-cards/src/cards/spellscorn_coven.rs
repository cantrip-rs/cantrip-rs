use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Return target spell to its owner's hand" (CR 400.3): not a counter, so a spell that can't be countered goes too, and a copy
/// that leaves the stack ceases to exist (CR 707.10a). With the target no longer legal nothing happens (CR 608.2b).
fn return_the_spell(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = targets::object(g, item, 0) else { return Ok(()) };
    g.remove_spell(spell, CounterTo::Hand);
    Ok(())
}

/// The Adventure (CR 715).
pub static TAKE_IT_BACK: CardDef = CardDef { spell: spell(&[targets::SPELL], return_the_spell), ..oracle::TAKE_IT_BACK };

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effects::each_opponent_discards_a_card, "When this creature enters, each opponent discards a card.")],
    adventure: Some(&TAKE_IT_BACK),
    ..oracle::SPELLSCORN_COVEN
};

pub static EXTRA: &[&CardDef] = &[&TAKE_IT_BACK];
