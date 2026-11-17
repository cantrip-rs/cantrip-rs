use cantrip_api::*;
use cantrip_mechanics::library::find_to_hand;
use cantrip_mechanics::select;
use cantrip_mechanics::triggers::ward;
use cantrip_oracle as oracle;

/// "Search your library for a basic land card or a Desert card, reveal it, put it into your hand, then shuffle. You gain 3
/// life." (CR 701.23, 119.1.) The Desert card is a card with the Desert subtype, the word read through the words of this
/// card's text (CR 612); the life is gained whether or not a card was found.
fn search_then_gain_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    find_to_hand(g, item.controller, item.object, select::BASIC_LAND_OR_DESERT_CARD, true)?;
    g.gain_life(item.controller, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<3>("Ward {3} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {3}.)"),
        ability(cost("{1}{G}").and(DISCARD_THIS), search_then_gain_life, "{1}{G}, Discard this card: Search your library for a basic land card or a Desert card, reveal it, put it into your hand, then shuffle. You gain 3 life.")
            .from(ZoneKind::Hand),
    ],
    ..oracle::SPINEWOODS_ARMADILLO
};
