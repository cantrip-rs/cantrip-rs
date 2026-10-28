use cantrip_api::*;
use cantrip_mechanics::{recruit, targets};
use cantrip_oracle as oracle;

/// "Counter target spell. If that spell's mana value was 2 or less, recruit." (CR 701.6, 701.70a): the mana value is the
/// spell's as it was on the stack (CR 608.2h), and the recruiting follows whether or not it could be countered; only an
/// illegal target stops the whole spell (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = targets::object(g, item, 0) else { return Ok(()) };
    let small = g.mana_value(spell) <= 2;
    g.counter(spell);
    if small {
        recruit::recruit(g, item.controller, item.object)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::SPELL], effect), ..oracle::SOUND_THE_TRUMPETS };

pub static EXTRA: &[&CardDef] = &[&cantrip_mechanics::tokens::HUMAN_SOLDIER];
