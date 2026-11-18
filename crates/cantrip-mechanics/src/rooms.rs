//! Rooms (CR 709.5): "the number of unlocked doors among Rooms you control". Misty Salon and Rampaging Soulrager count it.

use cantrip_api::*;

/// The number of unlocked doors among the Rooms `source`'s controller controls (CR 709.5, 709.5c), as an effect reads it: a
/// Room with both doors unlocked counts two, one with a door still locked counts one. "Room" is read through the words of
/// `source`'s text (CR 612).
pub fn unlocked_doors(q: &dyn Query, source: ObjectId) -> u32 {
    count(q.state(), q.controller(source), q.words(source).subtype(Subtype::Room), |id| q.chars(id))
}

/// The same count during a continuous effect (a static ability's condition), each Room as the layers before it made it.
pub fn unlocked_doors_in_layers(view: &dyn LayerQuery, source: ObjectId) -> u32 {
    count(view.state(), view.state().obj(source).controller, view.words(source).subtype(Subtype::Room), |id| view.chars(id))
}

fn count(state: &GameState, you: PlayerId, room: Subtype, chars: impl Fn(ObjectId) -> Chars) -> u32 {
    state.battlefield().iter().filter(|&&id| state.obj(id).controller == you && chars(id).has_subtype(room)).map(|&id| state.obj(id).marks.doors.count_ones()).sum()
}
