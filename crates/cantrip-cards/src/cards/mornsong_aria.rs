use cantrip_api::*;
use cantrip_mechanics::statics::players_cant_gain_life;
use cantrip_mechanics::{library, select, triggers};
use cantrip_oracle as oracle;

/// "Players can't draw cards" (CR 121.1, 101.2): every player, its controller included, however many cards. A spell or ability
/// that would draw still resolves and does its other things (the ruling).
fn players_cant_draw(_: &dyn Query, _: &Asker, _: PlayerId, _: u32) -> bool {
    true
}

/// "That player loses 3 life, searches their library for a card, puts it into their hand, then shuffles" (CR 119.3, 701.23): the
/// player whose draw step it is does all of it, the life lost first.
fn lose_life_and_tutor(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(player) = item.subject else { return Ok(()) };
    if !g.state().in_game(player) {
        return Ok(());
    }
    g.lose_life(player, 3);
    g.end_sentence();
    library::find_to_hand(g, player, item.object, select::CARD, false)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    cant_draw: &[players_cant_draw],
    cant_gain_life: &[players_cant_gain_life],
    abilities: &[triggered(
        triggers::EACH_PLAYERS_DRAW_STEP,
        lose_life_and_tutor,
        "At the beginning of each player's draw step, that player loses 3 life, searches their library for a card, puts it into their hand, then shuffles.",
    )],
    ..oracle::MORNSONG_ARIA
};
