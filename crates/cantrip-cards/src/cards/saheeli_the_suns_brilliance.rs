use cantrip_api::*;
use cantrip_mechanics::{copy, targets};
use cantrip_oracle as oracle;

/// "Create a token that's a copy of another target creature or artifact you control, except it's an artifact in addition to its
/// other types. It gains haste. Sacrifice it at the beginning of the next end step." (CR 707.2, 707.9b, 613.1f, 603.7c): the exception
/// is part of the copiable values (`with_type`); the haste is a separate effect on the token with no duration, so a copy of the token
/// does not copy it; nothing happens if the target is no longer legal (CR 608.2b).
fn copy_as_a_hasty_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(original) = targets::object(g, item, 0) else { return Ok(()) };
    let values = g.copiable_values(original).with_type(CardType::Artifact);
    copy::create_token_copy_with_haste_sacrificed_at_end_step(g, item, values, Duration::Indefinite, 0)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{U}{R}, {T}"),
        copy_as_a_hasty_artifact,
        "{U}{R}, {T}: Create a token that's a copy of another target creature or artifact you control, except it's an artifact in addition to its other types. It gains haste. Sacrifice it at the beginning of the next end step.",
    )
    .targets(&[targets::ANOTHER_ARTIFACT_OR_CREATURE_YOU_CONTROL])],
    delayed: &[copy::SACRIFICE_IT_AT_END_STEP],
    ..oracle::SAHEELI_THE_SUNS_BRILLIANCE
};
