use cantrip_api::*;
use cantrip_mechanics::manifest_dread::controller_manifests_dread;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // As an additional cost to cast this spell, exile six cards from your graveyard.
    additional_cost: Some(Cost { exile_from_graveyard: 6, ..Cost::NONE }),
    abilities: &[triggered(cantrip_mechanics::triggers::EACH_OPPONENTS_UPKEEP, controller_manifests_dread, "At the beginning of each opponent's upkeep, manifest dread. (Look at the top two cards of your library. Put one onto the battlefield face down as a 2/2 creature and the other into your graveyard. Turn it face up any time for its mana cost if it's a creature card.)")],
    ..oracle::ABHORRENT_OCULUS
};
