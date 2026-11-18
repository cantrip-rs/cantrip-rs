use cantrip_api::*;
use cantrip_mechanics::{lands, targets, triggers};
use cantrip_oracle as oracle;

fn return_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    lands::return_target_land_card_from_graveyard_tapped(g, item, 0)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_UPKEEP,
        return_land,
        "At the beginning of your upkeep, return target land card from your graveyard to the battlefield tapped.",
    )
    .targets(&[targets::LAND_CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::UNDERGROWTH_RECON
};
