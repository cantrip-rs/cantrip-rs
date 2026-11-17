use cantrip_api::*;
use cantrip_mechanics::{copy, targets};
use cantrip_oracle as oracle;

/// "Create a token that's a copy of target artifact or creature you control, except it's an artifact in addition to its other types.
/// It gains haste until end of turn. Sacrifice it at the beginning of the next end step." (CR 707.2, 707.9b, 611.2a, 603.7c): the
/// exception is part of the copiable values (`with_type`), so the copy of an artifact is simply that artifact; nothing happens if the
/// target is no longer legal (CR 608.2b).
fn duplicate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(original) = targets::object(g, item, 0) else { return Ok(()) };
    let values = g.copiable_values(original).with_type(CardType::Artifact);
    copy::create_token_copy_with_haste_sacrificed_at_end_step(g, item, values, Duration::EndOfTurn, 0)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT_OR_CREATURE_YOU_CONTROL], duplicate),
    delayed: &[copy::SACRIFICE_IT_AT_END_STEP],
    ..oracle::MOLTEN_DUPLICATION
};
