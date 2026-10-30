use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "Destroy target creature. Its controller creates two 1/1 red Officer creature tokens." The controller is read
/// before the destroy (CR 608.2h), and the tokens are created whether or not it was destroyed; an illegal target
/// creates nothing (CR 608.2b).
fn surgical_separation(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let controller = g.controller(creature);
    g.destroy(creature);
    tokens::create(g, controller, &tokens::OFFICER, 2)?;
    Ok(())
}

/// "You draw two cards and each player loses 2 life." Every player in the game loses the life, in turn order from
/// the active player (CR 101.4), as one instruction.
fn remain_fused(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 2);
    let players: Vec<PlayerId> = g.state().apnap().collect();
    instruction(g, |g| {
        for player in players {
            g.lose_life(player, 2);
        }
    });
    Ok(())
}

/// Choose one (CR 700.2); only the first mode has a target (CR 700.2c).
static MODAL: ModalDef = modes(&[
    mode(surgical_separation, "Destroy target creature. Its controller creates two 1/1 red Officer creature tokens.")
        .targets(&[targets::CREATURE]),
    mode(remain_fused, "You draw two cards and each player loses 2 life."),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::IMPOSSIBLE_CHOICE };

pub static EXTRA: &[&CardDef] = &[&tokens::OFFICER];
