use cantrip_api::*;
use cantrip_mechanics::library::{random_of, rest_to_bottom_in_random_order};
use cantrip_mechanics::top_of_library;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

/// "When this artifact is put into a graveyard from the battlefield, reveal the top thirteen cards of your library. Put a
/// random creature card from among them onto the battlefield. Put the rest on the bottom of your library in a random order"
/// (CR 701.20, 401.4). "Your" is the controller the artifact last had (CR 603.10a). A library of fewer than thirteen shows
/// what it has (CR 609.3); with no creature card among them nothing enters and none is drawn at random; the one chosen is
/// drawn from the game's own generator among the creature cards revealed; and the rest - every revealed card still in the
/// library - go to the bottom together.
fn reveal_and_put_a_random_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let revealed = top_of_library(g, you, 13);
    if revealed.is_empty() {
        return Ok(());
    }
    g.reveal(you, &revealed);
    g.end_sentence();
    let creatures: Vec<ObjectId> = revealed.iter().copied().filter(|&card| g.chars(card).is(CardType::Creature)).collect();
    if let Some(creature) = random_of(g, &creatures) {
        g.put_onto_battlefield(creature, you, false)?;
    }
    g.end_sentence();
    rest_to_bottom_in_random_order(g, you, &revealed);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_DIES,
        reveal_and_put_a_random_creature,
        "When this artifact is put into a graveyard from the battlefield, reveal the top thirteen cards of your library. Put a random creature card from among them onto the battlefield. Put the rest on the bottom of your library in a random order.",
    )],
    ..oracle::GETAWAY_BARREL
};
