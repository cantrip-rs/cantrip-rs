use cantrip_api::*;
use cantrip_mechanics::lands::{sub, word};
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever this creature or another Detective you control enters and whenever a Detective you control is turned face up" (CR 603.6a,
/// 701.40b, 603.2): this creature, or another Detective under your control as it entered (a face-down permanent has no creature types, so
/// it is not a Detective until it is turned up, CR 708.2a), and a Detective of yours turned face up, whoever turned it. Turning a
/// permanent face up is not entering the battlefield (CR 708.8), so no permanent triggers both.
fn detective_enters_or_is_turned_face_up(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::TurnedFaceUp { permanent, .. }
            if q.controller(*permanent) == me.controller && q.chars(*permanent).has_subtype(word(q, me.source, sub(Subtype::Detective))) =>
        {
            Fired::PLAIN
        }
        _ => triggers::this_or_another_of_subtype_you_control_enters::<{ sub(Subtype::Detective) }>(q, me, event),
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask(EventMask::ZONE_CHANGE.0 | EventMask::TURNED_FACE_UP.0), detective_enters_or_is_turned_face_up),
        effects::may_draw_then_discard,
        "Whenever this creature or another Detective you control enters and whenever a Detective you control is turned face up, you may draw a card. If you do, discard a card.",
    )],
    ..oracle::PROJEKTOR_INSPECTOR
};
