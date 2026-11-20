use cantrip_api::*;
use cantrip_mechanics::{library, mana, select};
use cantrip_oracle as oracle;

/// "Mill four cards, then return a creature card from your graveyard to the battlefield tapped."
fn mill_and_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::mill_then_return_a_card_to_the_battlefield_tapped(g, item, 4, select::creature_cards(select::as_many_as_can(1)))
}

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[
        mana_ability(TAP, add(Mana::U), "{T}: Add {U}."),
        ability(
            cost("{3}{U}{B}{B}").and(TAP).and(SACRIFICE_THIS),
            mill_and_return,
            "{3}{U}{B}{B}, {T}, Sacrifice this land: Mill four cards, then return a creature card from your graveyard to the battlefield tapped.",
        ),
    ],
    ..oracle::PORT_OF_KARFELL
};
