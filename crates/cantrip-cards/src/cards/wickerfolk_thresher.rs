use cantrip_api::*;
use cantrip_mechanics::{library, triggers, you_have_delirium};
use cantrip_oracle as oracle;

/// "Look at the top card of your library. If it's a land card, you may put it onto the battlefield. If you don't put the card onto the
/// battlefield, put it into your hand."
fn look_at_the_top_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::look_at_top_land_onto_battlefield_else_hand(g, item, false)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        look_at_the_top_card,
        "Delirium \u{2014} Whenever this creature attacks, if there are four or more card types among cards in your graveyard, look at the top card of your library. If it's a land card, you may put it onto the battlefield. If you don't put the card onto the battlefield, put it into your hand.",
    )
    .condition(you_have_delirium)],
    ..oracle::WICKERFOLK_THRESHER
};
