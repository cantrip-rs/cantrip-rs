use cantrip_api::*;
use cantrip_mechanics::{copy, statics, targets};
use cantrip_oracle as oracle;

/// "Another target artifact you control" (CR 115.1, 109.1): an artifact on the battlefield under the ability's controller,
/// other than its source.
fn another_artifact_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::ARTIFACT_YOU_CONTROL.filter)(q, asker, t) && t != Target::Object(asker.source)
}

/// "Create a token that's a copy of another target artifact you control. That token gains haste. Sacrifice it at the beginning
/// of the next end step" (CR 707.2, 613.1f, 603.7): the token copies exactly what was printed (the rulings), so its haste is an
/// effect on the token, not a copiable value a copy of it would copy (CR 707.2); the copy's "enters" abilities trigger and a
/// copied X is 0.
fn copy_with_haste(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    copy::create_token_copy_with_haste_sacrificed_at_end_step(g, item, target, 0)
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        statics::other_artifact_creatures_you_control,
        statics::plus_pt::<1, 0>,
        "Other artifact creatures you control get +1/+0.",
    )],
    abilities: &[ability(
        cost("{5}"),
        copy_with_haste,
        "{5}: Create a token that's a copy of another target artifact you control. That token gains haste. Sacrifice it at the beginning of the next end step.",
    )
    .targets(&[target(another_artifact_you_control)])],
    delayed: &[copy::SACRIFICE_IT_AT_END_STEP],
    ..oracle::CHROME_DOME
};
