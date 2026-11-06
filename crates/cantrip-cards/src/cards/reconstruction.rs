use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Return target artifact card from your graveyard to your hand." (CR 115.1).
fn return_target_artifact_card_graveyard_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Artifact])
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[target(return_target_artifact_card_graveyard_target_0)],
        effects::return_first_target_to_hand,
    ),
    ..oracle::RECONSTRUCTION
};
