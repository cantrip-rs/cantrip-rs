use cantrip_api::*;
use cantrip_mechanics::{bargain, pump, targets};
use cantrip_oracle as oracle;

/// "Target creature gets +2/+2 until end of turn. If this spell was bargained, that creature also gains flying and lifelink
/// until end of turn." The creature is the spell's one target, so with it illegal the spell does not resolve (CR 608.2b); the
/// pump (layer 7c) and the keywords (layer 6) are continuous effects for the rest of the turn (CR 611.2a, 613.1f), the keywords
/// the second sentence's one instruction.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    pump(g, item.object, creature, 2, 2);
    g.end_sentence();
    if bargain::bargained(item) {
        g.add_effect(item.object, creature, EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying, Keyword::Lifelink])), Duration::EndOfTurn);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    optional_cost: Some(bargain::OPTIONAL_COST),
    labels: bargain::LABELS,
    ..oracle::ARCHONS_GLORY
};
