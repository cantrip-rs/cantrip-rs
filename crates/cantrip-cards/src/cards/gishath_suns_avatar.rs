use cantrip_api::*;
use cantrip_mechanics::select::{self, Selection};
use cantrip_mechanics::{choose, library, triggers};
use cantrip_oracle as oracle;

/// "Whenever Gishath deals combat damage to a player, reveal that many cards from the top of your library. Put any number of
/// Dinosaur creature cards from among them onto the battlefield and the rest on the bottom of your library in a random order"
/// (CR 510.2, 701.20, 401.4, 608.2d): "that many" is the combat damage dealt, and a library with fewer cards reveals what it has
/// (CR 609.3). All the revealed cards are shown and the player takes none, some or all of the Dinosaur creature cards, which enter
/// together; every revealed card that did not, a Dinosaur left behind included, goes to the bottom. "Dinosaur" is read through the
/// source's words (CR 612), a creature type a text-changing effect can change.
fn reveal_and_put_dinosaurs_onto_the_battlefield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let revealed = cantrip_mechanics::top_of_library(g, you, item.matched_damage_amount() as usize);
    g.reveal(you, &revealed);
    let rule = Selection::new(select::ANY_NUMBER).with(select::CREATURE).with(select::subtype_card(Subtype::Dinosaur)).called("Dinosaur creature");
    let any = cantrip_api::select::first_answer(&*g, &rule, you, item.object, &revealed, true).is_some_and(|taken| !taken.is_empty());
    let chosen = if any { choose::select(g, you, item.object, Purpose::PutOntoBattlefield, revealed.clone(), rule)? } else { Vec::new() };
    g.put_all_onto_battlefield(&chosen.into_iter().map(|card| (card, you)).collect::<Vec<_>>())?;
    library::rest_to_bottom_in_random_order(g, you, &revealed);
    Ok(())
}

/// Vigilance, trample and haste are core keywords.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT,
        reveal_and_put_dinosaurs_onto_the_battlefield,
        "Whenever Gishath deals combat damage to a player, reveal that many cards from the top of your library. Put any number of Dinosaur creature cards from among them onto the battlefield and the rest on the bottom of your library in a random order.",
    )
    .damage_amount()],
    ..oracle::GISHATH_SUNS_AVATAR
};
