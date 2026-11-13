use cantrip_api::*;
use cantrip_mechanics::statics::itself;
use cantrip_mechanics::select;
use cantrip_mechanics::{lands, library, sacrifice_chosen, triggers};
use cantrip_oracle as oracle;

/// "Search your library for a basic Mountain card, put it onto the battlefield tapped, then shuffle" (CR 701.23): the player
/// finds one basic land card with the Mountain subtype or none, and the library is shuffled either way. "Mountain" is a word
/// of Molten Man's text (CR 612), which the selection rule reads.
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_battlefield(g, item.controller, item.object, select::BASIC_MOUNTAIN_CARD, true)?;
    Ok(())
}

/// "Sacrifice a land" (CR 701.21a): its controller chooses one land they control, or the only one there is.
fn sacrifice_a_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    sacrifice_chosen(g, item.controller, item.object, 1, |c| c.is(CardType::Land)).map(drop)
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::PtModify, itself, lands::plus_per_subtype_you_control::<{ lands::sub(Subtype::Mountain) }, false, 1, 1>, "Molten Man gets +1/+1 for each Mountain you control.")],
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            search,
            "When Molten Man enters, search your library for a basic Mountain card, put it onto the battlefield tapped, then shuffle.",
        ),
        triggered(triggers::THIS_LEAVES, sacrifice_a_land, "When Molten Man leaves the battlefield, sacrifice a land."),
    ],
    ..oracle::MOLTEN_MAN_INFERNO_INCARNATE
};
