use cantrip_api::*;
use cantrip_mechanics::{choose, select, top_of_library};
use cantrip_oracle as oracle;

/// "Look at the top twenty cards of your library, put any number of land cards from among them onto the battlefield tapped, then
/// shuffle. You gain 8 life." (CR 401.2, 608.2c, 701.24, 119.1): a library of fewer than twenty shows what it has (CR 609.3); the
/// player sees every card and may take any number of the lands (none included), which enter together, tapped (CR 110.5b,
/// 603.6a); the rest stay in the library, which is then shuffled whether or not a land was taken. The life follows the
/// shuffle, a sentence of its own.
fn gate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let looked = top_of_library(g, you, 20);
    g.look_at(you, &looked);
    let rule = select::land_cards(select::ANY_NUMBER);
    let lands = choose::select(g, you, item.object, Purpose::PutOntoBattlefield, looked, rule)?;
    g.put_all_onto_battlefield_as(&lands.into_iter().map(|land| (land, you, true)).collect::<Vec<_>>())?;
    g.shuffle(you);
    g.end_sentence();
    g.gain_life(you, 8);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], gate), ..oracle::THROUGH_THE_FOREST_GATE };
