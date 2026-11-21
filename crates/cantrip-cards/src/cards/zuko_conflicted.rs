use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_FIRST_MAIN_PHASE;
use cantrip_mechanics::{choose, effects, history, targets};
use cantrip_oracle as oracle;

/// "... and you lose 2 life": done first, whichever mode is chosen.
fn lose_2_life(g: &mut dyn Game, item: &StackItem) {
    g.lose_life(item.controller, 2);
}

fn draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    lose_2_life(g, item);
    effects::draw(g, item, 1)
}

fn counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    lose_2_life(g, item);
    effects::plus_one_counter_on_this(g, item)
}

fn add_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    lose_2_life(g, item);
    g.add_mana(item.controller, Mana::R, 1);
    Ok(())
}

/// "Exile Zuko, then return him to the battlefield under an opponent's control" (CR 400.7, 110.2a): the controller chooses the
/// opponent when there are several. It comes back a new object, with nothing chosen yet (CR 400.7), and nothing is exiled once Zuko has
/// left the battlefield. A token exiled this way is gone for good (CR 111.8).
fn give_to_an_opponent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    lose_2_life(g, item);
    if !targets::permanent(g, item.object) {
        return Ok(());
    }
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    let opponent = match opponents.len() {
        0 => return Ok(()),
        1 => opponents[0],
        _ => match choose::player_about(g, item.controller, item.object, opponents, "GainControlOf", item.object)? {
            Some(opponent) => opponent,
            None => return Ok(()),
        },
    };
    for exiled in g.exile_all(&[item.object]) {
        if !g.obj(exiled).token && g.obj(exiled).zone.is(ZoneKind::Exile) {
            g.put_all_onto_battlefield(&[(exiled, opponent)])?;
        }
    }
    Ok(())
}

/// "Choose one that hasn't been chosen" (CR 700.2): a mode already chosen for this permanent's ability is not offered again, and the
/// ability is removed from the stack as it would be put there once all four have been chosen (CR 603.3c).
static CHOICE: ModalDef = modes(&[
    mode(draw, "Draw a card."),
    mode(counter, "Put a +1/+1 counter on Zuko."),
    mode(add_red, "Add {R}."),
    mode(give_to_an_opponent, "Exile Zuko, then return him to the battlefield under an opponent's control."),
])
    .allowed(history::mode_not_chosen_before);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        YOUR_FIRST_MAIN_PHASE,
        &CHOICE,
        "At the beginning of your first main phase, choose one that hasn't been chosen and you lose 2 life \u{2014} draw a card; or put a +1/+1 counter on Zuko; or add {R}; or exile Zuko, then return him to the battlefield under an opponent's control.",
    )],
    ..oracle::ZUKO_CONFLICTED
};
