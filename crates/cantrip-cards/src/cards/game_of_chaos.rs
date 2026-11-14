use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

const AGAIN: Question = Question::new(
    "Game of Chaos", "flip again?", "The life stakes will double", "Flip again", "Stop",
).no_default();

/// How far from zero this card takes a life total (RX-8, beyond practical scale): a flip is offered again only while
/// its doubled stake, gained or lost by either player, leaves both life totals within this many points of zero. It is
/// reached only after some 23 flips from 20 life, and keeps every total far inside the engine's 32-bit life with room
/// for later play: the difference between two totals (an exchange, Mirror Universe), and doublers such as Boon
/// Reflection or Bloodletter of Aclazotz applied to the last stake.
const LIFE_BOUND: i64 = 1 << 24;

/// Flip, change both players' life simultaneously, and let the winner decide whether to continue (CR 705.1, 101.4).
fn play(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let mut stake = 1i32;
    loop {
        let won = g.flip_coin(item.controller);
        let (winner, loser) = if won { (item.controller, opponent) } else { (opponent, item.controller) };
        // Each change is its own occurrence, replaced on its own and announced together (CR 101.4, 119.3).
        g.together(Together { life: &[(winner, stake), (loser, -stake)], ..Together::default() });
        let next = 2 * i64::from(stake);
        let fits = [item.controller, opponent].iter().all(|&p| i64::from(g.state().player(p).life).abs() + next <= LIFE_BOUND);
        if !fits || !effects::ask_may(g, winner, item.object, Purpose::Ask(&AGAIN))? { break; }
        stake *= 2;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::OPPONENT], play),
    ..oracle::GAME_OF_CHAOS
};
