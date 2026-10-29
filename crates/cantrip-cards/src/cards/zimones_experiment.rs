use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_mechanics::library::rest_to_bottom_in_random_order;
use cantrip_mechanics::top_of_library;
use cantrip_oracle as oracle;

/// "Look at the top five cards of your library. You may reveal up to two creature and/or land cards from among them, then
/// put the rest on the bottom of your library in a random order. Put all land cards revealed this way onto the battlefield
/// tapped and put all creature cards revealed this way into your hand." (CR 401.2, 701.20): the cards revealed are chosen
/// together; a land creature card is both, so it goes to the battlefield and not also to the hand (it has left the
/// library). The rest are placed before the revealed cards move.
fn experiment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let looked = top_of_library(g, item.controller, 5);
    if looked.is_empty() {
        return Ok(());
    }
    g.look_at(item.controller, &looked);
    let permitted: Vec<ObjectId> = looked.iter().copied().filter(|&c| g.chars(c).is(CardType::Creature) || g.chars(c).is(CardType::Land)).collect();
    let chosen = choose::cards_from_kind(g, item.controller, item.object, Purpose::Reveal, "creature and/or land", looked.clone(), permitted, 2, true)?;
    if !chosen.is_empty() {
        g.reveal(item.controller, &chosen);
    }
    let lands: Vec<ObjectId> = chosen.iter().copied().filter(|&c| g.chars(c).is(CardType::Land)).collect();
    let creatures: Vec<ObjectId> = chosen.iter().copied().filter(|&c| !lands.contains(&c)).collect();
    rest_to_bottom_in_random_order(g, item.controller, &looked.iter().copied().filter(|c| !chosen.contains(c)).collect::<Vec<_>>());
    let entering: Vec<(ObjectId, PlayerId, bool)> = lands.into_iter().map(|c| (c, item.controller, true)).collect();
    g.put_all_onto_battlefield_as(&entering)?;
    g.return_to_hand_revealed(&creatures);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], experiment), ..oracle::ZIMONES_EXPERIMENT };
