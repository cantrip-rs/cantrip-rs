use cantrip_api::*;
use cantrip_mechanics::lands::search_basic_land_to_battlefield;
use cantrip_mechanics::manifest_dread::controller_manifests_dread;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Whenever a face-down permanent you control enters" (CR 603.6a, 708.3): any permanent that entered face down (a manifest, a cloak,
/// a disguise or morph cast face down) under your control as it entered, this one's own manifest included - read from the
/// `EnteredFaceDown` event recorded with its move, not from the permanent as it is now.
fn face_down_permanent_you_control_enters(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::EnteredFaceDown { permanent, controller } if *controller == me.controller => Fired::about(Target::Object(*permanent)),
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, controller_manifests_dread, "When this enchantment enters, manifest dread."),
        triggered(
            when_asked(EventMask::ENTERED_FACE_DOWN, face_down_permanent_you_control_enters),
            |g, item| search_basic_land_to_battlefield(g, item.controller, item.object, true),
            "Whenever a face-down permanent you control enters, search your library for a basic land card, put it onto the battlefield tapped, then shuffle.",
        ),
    ],
    ..oracle::THREATS_AROUND_EVERY_CORNER
};
