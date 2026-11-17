use cantrip_api::*;
use cantrip_mechanics::select;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{lands, library, statics};
use cantrip_oracle as oracle;

/// "Search your library for a basic land card or a Desert card, reveal it, put it into your hand, then shuffle." (CR 701.23.)
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_hand(g, item.controller, item.object, select::BASIC_LAND_OR_DESERT_CARD, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        search,
        "When this creature enters, search your library for a basic land card or a Desert card, reveal it, put it into your hand, then shuffle.",
    )],
    statics: &[static_ability(
        Layer::PtModify,
        statics::itself,
        lands::plus_per_subtype_you_control::<{ lands::sub(Subtype::Desert) }, false, 1, 1>,
        "This creature gets +1/+1 for each Desert you control.",
    )],
    ..oracle::OUTCASTER_GREENBLADE
};
