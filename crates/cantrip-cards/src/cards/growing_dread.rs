use cantrip_api::*;
use cantrip_mechanics::{effects, manifest_dread, triggers};
use cantrip_oracle as oracle;

/// "Whenever you turn a permanent face up" (CR 708.7, 603.2): the controller of the spell or ability that turned it up, or the
/// player who took the special action, turned the permanent face up, which is the trigger's subject ("it"). A permanent an
/// opponent turns up is not it (`Event::TurnedFaceUp`'s `player`).
fn you_turn_a_permanent_face_up(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::TurnedFaceUp { permanent, player } if player == me.controller => Some(Fired { subject: Target::Object(permanent), data: [0; 2] }),
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, manifest_dread::controller_manifests_dread, "When this enchantment enters, manifest dread."),
        triggered(
            when_asked(EventMask::TURNED_FACE_UP, you_turn_a_permanent_face_up),
            effects::plus_one_counter_on_that_permanent,
            "Whenever you turn a permanent face up, put a +1/+1 counter on it.",
        ),
    ],
    ..oracle::GROWING_DREAD
};
