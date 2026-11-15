use cantrip_api::*;
use cantrip_mechanics::lands::search_basic_land_to_battlefield;
use cantrip_mechanics::manifest_dread::controller_manifests_dread;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, controller_manifests_dread, "When this enchantment enters, manifest dread."),
        triggered(
            triggers::FACE_DOWN_PERMANENT_YOU_CONTROL_ENTERS,
            |g, item| search_basic_land_to_battlefield(g, item.controller, item.object, true),
            "Whenever a face-down permanent you control enters, search your library for a basic land card, put it onto the battlefield tapped, then shuffle.",
        ),
    ],
    ..oracle::THREATS_AROUND_EVERY_CORNER
};
