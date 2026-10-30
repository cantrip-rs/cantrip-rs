use cantrip_api::*;
use cantrip_mechanics::{choose, library, select, targets, triggers::THIS_ENTERS, vivid};
use cantrip_oracle as oracle;

/// "Vivid — When this creature enters, reveal cards from the top of your library until you reveal X permanent cards, where X is
/// the number of colors among permanents you control. Put any number of those permanent cards onto the battlefield, then put
/// the rest of the revealed cards on the bottom of your library in a random order" (CR 701.20, 207.2c, 105.2, 401.4): X is counted as
/// the ability resolves, the cards are revealed together down to the Xth permanent card (the whole library if it holds fewer),
/// the player takes none, some or all of those permanent cards, which enter together, and every other revealed card, a permanent
/// card left behind included, goes to the bottom.
fn reveal_and_put_onto_the_battlefield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let x = usize::from(vivid::count(g, you));
    let (permanents, others) = library::reveal_until_n(g, you, x, &|q, card| targets::is_permanent_card(&q.chars(card)));
    let shown: Vec<ObjectId> = permanents.iter().chain(&others).copied().collect();
    let rule = Selection::listed(select::ANY_NUMBER, &shown, permanents).called("permanent");
    let chosen = choose::select(g, you, item.object, Purpose::PutOntoBattlefield, shown.clone(), rule)?;
    g.put_all_onto_battlefield(&chosen.into_iter().map(|card| (card, you)).collect::<Vec<_>>())?;
    library::rest_to_bottom_in_random_order(g, you, &shown);
    Ok(())
}

/// Trample is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        reveal_and_put_onto_the_battlefield,
        "Vivid — When this creature enters, reveal cards from the top of your library until you reveal X permanent cards, where X is the number of colors among permanents you control. Put any number of those permanent cards onto the battlefield, then put the rest of the revealed cards on the bottom of your library in a random order.",
    )],
    ..oracle::AURORA_AWAKENER
};
