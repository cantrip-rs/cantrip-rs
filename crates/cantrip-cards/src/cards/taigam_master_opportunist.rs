use cantrip_api::*;
use cantrip_mechanics::{flurry, suspend};
use cantrip_oracle as oracle;

/// "Copy it, then exile the spell you cast with four time counters on it. If it doesn't have suspend, it gains suspend." The
/// copy is made first and is not cast (CR 707.10); it is made from the spell's last-known information if the spell has
/// already left the stack, and the original is exiled only if it is still there (CR 400.7, 608.2b). The exiled card gains
/// suspend from this ability (CR 702.62a, 611.2a) for as long as it stays in exile; a card that has suspend already keeps its
/// own, and one that gets it twice has it once.
fn copy_then_suspend(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = item.subject.object() else { return Ok(()) };
    g.copy_spell(spell, item.controller, false)?;
    if g.obj(spell).became.is_some() {
        return Ok(());
    }
    let exiled = suspend::exile_with_time_counters(g, spell, 4).filter(|&card| !g.chars(card).keywords.has(Keyword::Suspend));
    if let Some(card) = exiled {
        g.add_effect(item.object, card, EffectKind::AddKeywords(Keywords::of(&[Keyword::Suspend])), Duration::Indefinite);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[flurry::trigger(
        "Flurry \u{2014} Whenever you cast your second spell each turn, copy it, then exile the spell you cast with four time counters on it. If it doesn't have suspend, it gains suspend.",
        copy_then_suspend,
    )],
    ..oracle::TAIGAM_MASTER_OPPORTUNIST
};
