use cantrip_api::*;
use cantrip_mechanics::{delayed, targets};
use cantrip_oracle as oracle;

/// "Counter target spell. If you control a Wizard, add an amount of {C} equal to the amount of mana spent to cast that spell at
/// the beginning of your next main phase." The Wizard is looked for as this resolves; the mana is owed whether or not the spell
/// could be countered, and only an illegal target stops the whole spell (CR 608.2b).
fn sculpt(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = cantrip_mechanics::counter_target(g, item, 0) else { return Ok(()) };
    let wizard = g.words(item.object).subtype(Subtype::Wizard);
    if g.permanents_of(item.controller).into_iter().any(|id| g.chars(id).has_subtype(wizard)) {
        delayed::at_your_next_main_phase(g, item, 0, Target::Object(spell));
    }
    Ok(())
}

/// "An amount of {C} equal to the amount of mana spent to cast that spell": what was spent on it, read from the spell as it was
/// on the stack (CR 608.2h), so a cost reduction or an {X} counts as paid.
fn add_mana(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let spent = item.subject.object().map_or(0, |spell| g.obj(spell).spent.total());
    if spent > 0 {
        g.add_mana(item.controller, Mana::C, spent);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::SPELL], sculpt),
    delayed: &[delayed_trigger(add_mana, "Add an amount of {C} equal to the amount of mana spent to cast that spell.")],
    ..oracle::MANA_SCULPT
};
