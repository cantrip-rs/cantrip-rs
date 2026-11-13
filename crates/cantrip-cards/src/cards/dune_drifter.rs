use cantrip_api::*;
use cantrip_mechanics::{crew::crew, effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Target artifact or creature card with mana value X or less from your graveyard" (CR 115.1, 107.3m, 202.3e): the X the Vehicle was
/// cast with, read from the permanent as the enters trigger goes on the stack; a Vehicle put onto the battlefield without being cast has X 0.
fn artifact_or_creature_card_with_mana_value_at_most_x(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard_with_mana_value_at_most(q, asker, t, &[CardType::Artifact, CardType::Creature], q.obj(asker.source).x)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::return_first_target_to_battlefield, "When this Vehicle enters, return target artifact or creature card with mana value X or less from your graveyard to the battlefield.")
            .targets(&[target(artifact_or_creature_card_with_mana_value_at_most_x)]),
        crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::DUNE_DRIFTER
};
