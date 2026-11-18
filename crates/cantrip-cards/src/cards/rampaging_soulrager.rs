use cantrip_api::*;
use cantrip_mechanics::rooms;
use cantrip_mechanics::statics::plus_pt;
use cantrip_oracle as oracle;

/// "... as long as there are two or more unlocked doors among Rooms you control" (CR 611.3a, 709.5) as a static ability's
/// `StaticDef::affects`: only the source itself, asked again whenever characteristics are computed, so it turns on with the
/// unlock and off if a Room leaves.
fn itself_with_two_unlocked_doors_among_your_rooms(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && rooms::unlocked_doors_in_layers(view, source) >= 2
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
