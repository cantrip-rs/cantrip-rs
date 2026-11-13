use cantrip_api::*;
use cantrip_mechanics::{effects, lands};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Return all land cards from your graveyard to the battlefield tapped."
fn return_lands(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    lands::return_all_land_cards_from_graveyard_tapped(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::you_mill::<3>, "When this creature enters, mill three cards."),
        ability(
            cost("{3}{G}").and(SACRIFICE_THIS),
            return_lands,
            "{3}{G}, Sacrifice this creature: Return all land cards from your graveyard to the battlefield tapped.",
        ),
    ],
    ..oracle::AFTERMATH_ANALYST
};
