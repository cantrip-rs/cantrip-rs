use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_this;
use cantrip_oracle as oracle;

/// "Whenever a face-down permanent you control enters and whenever this creature or another permanent you control is turned face
/// up" (CR 708.2, 701.40b, 603.2): one trigger for each permanent that entered the battlefield face down under the source's
/// controller's control (as it entered, CR 708.3), and one for each permanent, this creature included, that is turned face up while
/// the source's controller controls it, whoever turned it up. A face-down Cryptid Inspector has no abilities, so it triggers only
/// from the turning up of itself.
fn face_down_enters_or_permanent_turned_face_up(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    let you = q.controller(source);
    match event {
        Event::EnteredFaceDown { controller, .. } if *controller == you => Fired::PLAIN,
        Event::TurnedFaceUp { permanent, .. } if q.controller(*permanent) == you => Fired::PLAIN,
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask(EventMask::ENTERED_FACE_DOWN.0 | EventMask::TURNED_FACE_UP.0), face_down_enters_or_permanent_turned_face_up),
        plus_one_counter_on_this,
        "Whenever a face-down permanent you control enters and whenever this creature or another permanent you control is turned face up, put a +1/+1 counter on this creature.",
    )],
    ..oracle::CRYPTID_INSPECTOR
};
