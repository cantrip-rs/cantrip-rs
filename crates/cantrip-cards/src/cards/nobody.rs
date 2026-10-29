use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, scry};
use cantrip_oracle as oracle;

/// "Other artifact you control" (CR 115.1, 109.1): an artifact on the battlefield under the ability's controller, not its source.
fn other_artifact_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if id != asker.source && q.state().battlefield().contains(&id)
        && q.controller(id) == asker.controller && q.chars(id).is(CardType::Artifact))
}

/// "Return up to one other target artifact you control to its owner's hand. Scry 1." The scry is a separate instruction: it happens
/// whether or not a target was chosen, or is still legal (CR 608.2b).
fn return_an_artifact_and_scry(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_first_target_to_hand(g, item)?;
    scry(g, item.controller, item.object, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        return_an_artifact_and_scry,
        "When this creature enters, return up to one other target artifact you control to its owner's hand. Scry 1.",
    )
    .targets(&[up_to_one(other_artifact_you_control)])],
    ..oracle::NOBODY
};
