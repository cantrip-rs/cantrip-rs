use cantrip_api::*;
use cantrip_mechanics::{copy, statics, targets};
use cantrip_oracle as oracle;

/// "Another target artifact you control" (CR 115.1, 109.1): an artifact on the battlefield under the ability's controller,
/// other than its source.
fn another_artifact_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if id != asker.source && targets::permanent(q, id) && q.controller(id) == asker.controller && q.chars(id).is(CardType::Artifact))
}

/// "Create a token that's a copy of another target artifact you control. That token gains haste. Sacrifice it at the beginning
/// of the next end step" (`copy::create_hasty_token_copy_sacrificed_at_end_step`, CR 707.2, 603.7): the copy's "enters"
/// abilities trigger (the rulings) and a copied X is 0.
fn copy_with_haste(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let me = g.card_id(&CARD);
    copy::create_hasty_token_copy_sacrificed_at_end_step(g, item, me, 0)
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
