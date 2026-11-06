use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Target spell or permanent becomes green indefinitely (CR 105.3, 611.2a, 400.7a).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(object) = targets::object(g, item, 0) else { return Ok(()) };
    let colors = g.words(item.object).colors(ColorSet::of(Color::Green));
    g.add_effect(item.object, object, EffectKind::SetColors(colors), Duration::Indefinite);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::SPELL_OR_PERMANENT], effect),
    ..oracle::LIFELACE
};
