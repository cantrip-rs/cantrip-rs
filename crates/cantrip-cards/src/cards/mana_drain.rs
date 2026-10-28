use cantrip_api::*;
use cantrip_mechanics::{delayed, targets};
use cantrip_oracle as oracle;

/// Counter target spell. At the beginning of your next main phase, add {C} equal to that spell's mana value. The mana is
/// owed whether or not the spell could be countered; only an illegal target stops the whole spell (CR 608.2b).
fn drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = cantrip_mechanics::counter_target(g, item, 0) else { return Ok(()) };
    delayed::at_your_next_main_phase(g, item, 0, Target::Object(spell));
    Ok(())
}

/// "Add an amount of {C} equal to that spell's mana value": the spell's, as it was on the stack (CR 608.2h).
fn add_mana(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(spell) = item.subject {
        let value = g.mana_value(spell);
        if value > 0 {
            g.add_mana(item.controller, Mana::C, value);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::SPELL], drain),
    delayed: &[delayed_trigger(add_mana, "Add an amount of {C} equal to that spell's mana value.")],
    ..oracle::MANA_DRAIN
};
