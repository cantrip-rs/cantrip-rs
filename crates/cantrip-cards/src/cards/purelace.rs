use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// The target becomes the text-adjusted color indefinitely (CR 105.3, 611.2a, 612.2).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(object) = targets::object(g, item, 0) else { return Ok(()) };
    let colors = g.words(item.object).colors(ColorSet::of(Color::White));
    g.add_effect(item.object, object, EffectKind::SetColors(colors), Duration::Indefinite);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::SPELL_OR_PERMANENT], effect),
    ..oracle::PURELACE
};
