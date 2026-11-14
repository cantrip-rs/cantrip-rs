use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

const AGAIN: Question = Question::new(
    "Game of Chaos", "flip again?", "The life stakes will double", "Flip again", "Stop",
).no_default();

/// Flip, change both players' life simultaneously, and let the winner decide whether to continue (CR 705.1, 101.4).
fn play(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let mut stake = 1u64;
    loop {
        let won = g.flip_coin(item.controller);
        let (winner, loser) = if won { (item.controller, opponent) } else { (opponent, item.controller) };
        g.life_changes_together(&[
            LifeChange { player: winner, magnitude: stake, negative: false },
            LifeChange { player: loser, magnitude: stake, negative: true },
        ]);
        if !effects::ask_may(g, winner, item.object, Purpose::Ask(&AGAIN))? { break; }
        stake *= 2;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::OPPONENT], play),
    ..oracle::GAME_OF_CHAOS
};
