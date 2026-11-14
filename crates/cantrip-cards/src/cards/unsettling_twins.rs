use cantrip_api::*;
use cantrip_mechanics::manifest_dread::controller_manifests_dread;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        controller_manifests_dread,
        "When this creature enters, manifest dread. (Look at the top two cards of your library. Put one onto the battlefield face down as a 2/2 creature and the other into your graveyard. Turn it face up any time for its mana cost if it's a creature card.)",
    )],
    ..oracle::UNSETTLING_TWINS
};
