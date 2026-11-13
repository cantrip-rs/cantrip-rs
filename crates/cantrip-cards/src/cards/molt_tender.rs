use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Mill a card." (CR 701.17a): the top card of the controller's library goes to their graveyard.
fn mill_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.mill(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, mill_a_card, "{T}: Mill a card. (Put the top card of your library into your graveyard.)"),
        mana_ability(
            TAP.and(exile_from_graveyard(1)),
            add_any_color(1),
            "{T}, Exile a card from your graveyard: Add one mana of any color.",
        ),
    ],
    ..oracle::MOLT_TENDER
};
