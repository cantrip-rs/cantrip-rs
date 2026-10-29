use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::{exile_and_return, targets};
use cantrip_oracle as oracle;

fn artifact_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id)
        && q.controller(id) == asker.controller && q.chars(id).is(CardType::Artifact))
}

/// "Exile up to one target artifact you control and up to one target creature you control. Then return them to the battlefield under
/// their owners' control" (CR 608.2c, 400.7): the two are exiled together and come back together as new objects; each target is
/// checked on its own (CR 608.2b), and one permanent chosen for both words is exiled and returned once. A token ceases to exist in
/// exile (CR 111.7).
fn blink_an_artifact_and_a_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut chosen = Vec::new();
    for slot in 0..2 {
        if let Some(permanent) = targets::object(g, item, slot) {
            if !chosen.contains(&permanent) {
                chosen.push(permanent);
            }
        }
    }
    exile_and_return(g, &chosen)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        blink_an_artifact_and_a_creature,
        "At the beginning of your end step, exile up to one target artifact you control and up to one target creature you control. Then return them to the battlefield under their owners' control.",
    )
    .targets(&[up_to_one(artifact_you_control), up_to_one(targets::CREATURE_YOU_CONTROL.filter)])],
    ..oracle::DON_LEO_PROBLEM_SOLVERS
};
