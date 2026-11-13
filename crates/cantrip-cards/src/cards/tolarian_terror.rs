use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, triggers};
use cantrip_oracle as oracle;

/// "This spell costs {1} less to cast for each instant and sorcery card in your graveyard" (CR 601.2f, 118.7a): generic mana only, so it
/// never goes below {U}. Ward {2} (CR 702.21).
pub static CARD: CardDef = CardDef {
    own_cost_changes: &[cost_reduction::for_each_instant_and_sorcery_in_your_graveyard],
    abilities: &[triggers::ward::<2>(
        "Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)",
    )],
    ..oracle::TOLARIAN_TERROR
};
