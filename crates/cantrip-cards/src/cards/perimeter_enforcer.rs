use cantrip_api::*;
use cantrip_mechanics::{effects, lands, triggers};
use cantrip_oracle as oracle;

/// "Whenever another Detective you control enters and whenever a Detective you control is turned face up" (CR 603.6a,
/// 708.8): once for each other permanent that is a Detective (a token too, the type read through the words of this card's text,
/// CR 612) entering under the source's controller's control, and once for each permanent that is a Detective once it is face up
/// and is controlled by the source's controller when it is turned up, whoever turns it. A permanent that enters face down is not
/// turned up by entering, and a face-down permanent is no Detective (CR 708.2) until it is turned up.
fn another_detective_enters_or_detective_turned_face_up(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Detective) }>(q, me, event)
        .or_else(|| triggers::of_subtype_you_control_turned_face_up::<{ lands::sub(Subtype::Detective) }>(q, me, event))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask(EventMask::ZONE_CHANGE.0 | EventMask::TURNED_FACE_UP.0), another_detective_enters_or_detective_turned_face_up),
        effects::this_gets::<1, 1>,
        "Whenever another Detective you control enters and whenever a Detective you control is turned face up, this creature gets +1/+1 until end of turn.",
    )],
    ..oracle::PERIMETER_ENFORCER
};
