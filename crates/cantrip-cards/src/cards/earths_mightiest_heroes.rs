use cantrip_api::*;
use cantrip_mechanics::{choose, select, teamwork, top_of_library};
use cantrip_oracle as oracle;

/// "Reveal the top eight cards of your library. You may put a creature card from among them onto the battlefield. If this spell
/// was cast using teamwork, put any number of creature cards from among them onto the battlefield instead. Put the rest into your
/// graveyard." (CR 701.20, 702.194b, 400.3): the eight cards (fewer from a smaller library, CR 701.20a) are revealed to every
/// player, the creature cards among them are chosen - one at most, or any number with teamwork, the "instead" replacing the one - and
/// enter together, and whatever is left of the revealed cards goes into the graveyard together, in the order their owner arranges
/// them (CR 404.3). A copy of a spell cast using teamwork was cast that way too (CR 707.10).
fn reveal_and_put_creatures_onto_the_battlefield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let revealed = top_of_library(g, you, 8);
    if revealed.is_empty() {
        return Ok(());
    }
    g.reveal(you, &revealed);
    let rule = if teamwork::used(item) { select::creature_cards(select::ANY_NUMBER) } else { select::CREATURE_CARD };
    let chosen = choose::select(g, you, item.object, Purpose::PutOntoBattlefield, revealed.clone(), rule)?;
    if !chosen.is_empty() {
        let entering: Vec<(ObjectId, PlayerId, bool)> = chosen.iter().map(|&card| (card, you, false)).collect();
        g.put_all_onto_battlefield_as(&entering)?;
    }
    g.end_sentence();
    // The rest: the revealed cards that are still in the library (CR 400.7), put into the graveyard.
    let rest: Vec<ObjectId> = revealed.into_iter().filter(|card| !chosen.contains(card)).collect();
    g.put_into_graveyard(&rest);
    Ok(())
}

/// Teamwork 5 (as an additional cost to cast this spell, you may tap any number of creatures you control with total power 5 or
/// more): an optional cost announced before the spell's choices.
pub static CARD: CardDef = CardDef {
    spell: spell(&[], reveal_and_put_creatures_onto_the_battlefield),
    optional_cost: Some(teamwork::optional(5)),
    labels: &["Cast without teamwork", "Teamwork 5: Tap any number of creatures you control with total power 5 or more"],
    ..oracle::EARTHS_MIGHTIEST_HEROES
};
