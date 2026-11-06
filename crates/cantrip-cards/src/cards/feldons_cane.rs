use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_oracle as oracle;

/// Shuffle the controller's graveyard into their library (CR 701.24c).
fn shuffle_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::shuffle_graveyard_into_library(g, item.controller);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP.and(EXILE_THIS),
        shuffle_graveyard,
        "{T}, Exile this artifact: Shuffle your graveyard into your library.",
    )],
    ..oracle::FELDONS_CANE
};
