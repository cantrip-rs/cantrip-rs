use cantrip_api::*;
use cantrip_mechanics::{counter_unless_pays, greatest_power, targets};
use cantrip_oracle as oracle;

/// Put X +1/+1 counters on target creature you control. Then counter up to one target spell unless its controller pays
/// mana equal to the greatest power among creatures you control. In order (CR 608.2c): the counters go on first, so the
/// greatest power is read with them on, once, among the creatures its controller controls.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if item.x > 0
        && let Some(creature) = targets::object(g, item, 0)
    {
        g.add_counters(creature, CounterKind::PlusOne, item.x as i16);
    }
    if targets::object(g, item, 1).is_some() {
        let greatest = greatest_power::among_creatures_you_control(g, item.controller).min(255) as u8;
        counter_unless_pays(g, item, 1, generic(greatest))?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::SPELL.optional()], effect),
    ..oracle::REPULSIVE_MUTATION
};
