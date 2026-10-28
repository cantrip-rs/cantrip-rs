use cantrip_api::*;
use cantrip_mechanics::effects::gain_life;
use cantrip_mechanics::lands::may_search_basic_land_to_top;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this creature enters, you gain 2 life. You may search your library for a basic land card, reveal it, then shuffle and put
/// that card on top." (CR 119.3, 701.23): two sentences, so the life is gained before the search is offered (CR 608.2c). Flying
/// is in its content file.
fn gain_two_then_may_search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    gain_life(g, item, 2)?;
    g.end_sentence();
    may_search_basic_land_to_top(g, item.controller, item.object)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        gain_two_then_may_search,
        "When this creature enters, you gain 2 life. You may search your library for a basic land card, reveal it, then shuffle and put that card on top.",
    )],
    ..oracle::OLD_THRUSH
};
