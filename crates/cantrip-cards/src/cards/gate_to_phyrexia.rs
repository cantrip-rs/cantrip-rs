use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Sacrifice a creature: Destroy target artifact. Activate only during your upkeep and only once each turn." (CR 602.5).
fn destroy_target_artifact_activate_only_if(q: &dyn Query, asker: &Asker) -> bool {
    q.state().turn.active == asker.controller && q.state().turn.step() == Step::Upkeep
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                sacrifice(1, targets::filters::creature),
                effects::destroy_first_target,
                "Sacrifice a creature: Destroy target artifact. Activate only during your upkeep and only once each turn.",
            )
            .targets(&[targets::ARTIFACT])
            .once_each_turn()
            .condition(destroy_target_artifact_activate_only_if),
    ],
    ..oracle::GATE_TO_PHYREXIA
};
