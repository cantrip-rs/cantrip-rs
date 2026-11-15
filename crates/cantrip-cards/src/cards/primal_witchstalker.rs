use cantrip_api::*;
use cantrip_mechanics::{lands, library, targets};
use cantrip_oracle as oracle;

fn return_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    lands::return_target_land_card_from_graveyard_tapped(g, item, 0)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        cantrip_mechanics::triggers::THIS_ENTERS,
        library::mill_then_reflexive_trigger::<4>,
        "When this creature enters, mill four cards. When you do, return target land card from your graveyard to the battlefield tapped.",
    )],
    delayed: &[delayed_trigger(
        return_land,
        "When you do, return target land card from your graveyard to the battlefield tapped.",
    )
    .targets(&[targets::LAND_CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::PRIMAL_WITCHSTALKER
};
