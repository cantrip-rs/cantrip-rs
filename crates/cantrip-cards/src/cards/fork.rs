use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Copy the spell with red as its only color (CR 707.9b, 707.10).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = targets::object(g, item, 0) else { return Ok(()) };
    let values = g.copiable_values(spell)
        .colored(ColorSet::NONE.with(g.words(item.object).color(Color::Red)));
    g.copy_spell_as(spell, item.controller, true, values)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::INSTANT_OR_SORCERY_SPELL], effect),
    ..oracle::FORK
};
