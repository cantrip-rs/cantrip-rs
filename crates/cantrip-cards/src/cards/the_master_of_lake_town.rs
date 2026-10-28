use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

/// "Whenever a player loses life" (CR 119.3, 603.2): any player's, one event for each time life is lost, damage and paying
/// life included, and none when none is (a loss of 0, or damage that was prevented). The trigger is about the player and
/// carries how much was lost.
fn a_player_loses_life(_: &dyn Query, _: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::LifeChange { player, delta } if *delta < 0 => Some(Fired { subject: Target::Player(*player), data: [delta.unsigned_abs(), 0] }),
        _ => None,
    }
}

/// "That player mills that many cards" (CR 701.17a): the player who lost the life, whoever controls this creature.
fn that_player_mills(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = item.subject.player() {
        g.mill(player, item.data[0]);
    }
    Ok(())
}

/// "Draw a card for each graveyard with seven or more cards in it" (CR 121.1, 404.1): counted as the ability resolves, so
/// the cards the first ability milled, and this creature itself, are in them. A graveyard of a player who has left the game
/// is empty.
fn draw_for_big_graveyards(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let big = g.state().seats().filter(|&p| g.state().zone(Zone::graveyard(p)).len() >= 7).count();
    g.draw(item.controller, big as u32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::LIFE_CHANGE, a_player_loses_life),
            that_player_mills,
            "Whenever a player loses life, that player mills that many cards. (Damage causes loss of life.)",
        ),
        triggered(THIS_DIES, draw_for_big_graveyards, "When The Master of Lake-town dies, draw a card for each graveyard with seven or more cards in it."),
    ],
    ..oracle::THE_MASTER_OF_LAKE_TOWN
};
