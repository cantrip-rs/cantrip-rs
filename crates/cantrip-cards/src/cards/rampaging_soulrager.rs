use cantrip_api::*;
use cantrip_mechanics::statics::plus_pt;
use cantrip_oracle as oracle;

/// The number of unlocked doors among the Rooms `source`'s controller controls (CR 709.5, 709.5c): a Room with both doors unlocked
/// counts two, one with none (a half still locked, CR 709.5c) counts none. Each is judged as the layers made it.
fn unlocked_doors_among_your_rooms(view: &dyn LayerQuery, source: ObjectId) -> u32 {
    let state = view.state();
    let you = state.obj(source).controller;
    let room = view.words(source).subtype(Subtype::Room);
    state.battlefield().iter().filter(|&&id| state.obj(id).controller == you && view.chars(id).has_subtype(room)).map(|&id| state.obj(id).marks.doors.count_ones()).sum()
}

/// "... as long as there are two or more unlocked doors among Rooms you control" (CR 611.3a, 709.5) as a static ability's
/// `StaticDef::affects`: only the source itself, asked again whenever characteristics are computed, so it turns on with the
/// unlock and off if a Room leaves.
fn itself_with_two_unlocked_doors_among_your_rooms(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && unlocked_doors_among_your_rooms(view, source) >= 2
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        itself_with_two_unlocked_doors_among_your_rooms,
        plus_pt::<3, 0>,
        "This creature gets +3/+0 as long as there are two or more unlocked doors among Rooms you control.",
    )],
    ..oracle::RAMPAGING_SOULRAGER
};
