use cantrip_api::*;
use cantrip_mechanics::{effects, history, library, triggers};
use cantrip_oracle as oracle;

const SACRIFICE: Question = Question::new(
    "SacrificeServantOfTheStinger",
    "sacrifice this creature?",
    "If you do, search your library for a card, put it into your hand, then shuffle",
    "Sacrifice",
    "Don't sacrifice",
);

/// "If you've committed a crime this turn" (CR 603.4, 700.13): an intervening "if", asked as the creature deals the damage and
/// again as the ability resolves.
fn you_committed_a_crime_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    history::committed_a_crime_this_turn(q.history(), asker.controller)
}

/// "You may sacrifice this creature. If you do, search your library for a card, put it into your hand, then shuffle" (CR 701.21a,
/// 701.23, 608.2c): the search happens only if the creature was really sacrificed, which declining, or a creature that has left
/// the battlefield already, is not (CR 400.7).
fn sacrifice_and_search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.can_sacrifice(item.controller, item.object)
        || !effects::ask_may(g, item.controller, item.object, Purpose::Ask(&SACRIFICE))?
        || g.sacrifice(item.controller, &[item.object]).is_empty()
    {
        return Ok(());
    }
    g.end_sentence();
    library::search_for_a_card_to_hand(g, item)
}

// Deathtouch is the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER,
        sacrifice_and_search,
        "Whenever this creature deals combat damage to a player, if you've committed a crime this turn, you may sacrifice this creature. If you do, search your library for a card, put it into your hand, then shuffle. (Targeting opponents, anything they control, and/or cards in their graveyards is a crime.)",
    )
    .condition(you_committed_a_crime_this_turn)],
    ..oracle::SERVANT_OF_THE_STINGER
};
