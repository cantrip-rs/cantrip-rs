use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

const DRAW: Question = Question::new("Draw", "draw a card?", "Sibilant Spirit's attack ability", "Draw", "Don't draw").amount(1);

/// The defending player may draw, even if the attacker has left combat (CR 508.5, 113.7a).
fn defending_player_draw_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = g.defending_player(item.object).filter(|&p| g.state().in_game(p)) else { return Ok(()) };
    effects::may_draw(g, player, item.object, 1, &DRAW)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        defending_player_draw_card,
        "Whenever this creature attacks, defending player may draw a card.",
    )],
    ..oracle::SIBILANT_SPIRIT
};
