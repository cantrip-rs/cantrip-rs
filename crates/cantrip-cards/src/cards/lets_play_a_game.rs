use cantrip_api::*;
use cantrip_mechanics::{delirium, each_discards_cards, effects};
use cantrip_oracle as oracle;

/// "Creatures your opponents control get -1/-1 until end of turn" (CR 611.2c).
fn opponents_creatures_shrink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::pump_creatures_your_opponents_control(g, item, -1, -1);
    Ok(())
}

/// "Each opponent discards two cards" (CR 701.9, 101.4): chosen in turn order, discarded together.
fn each_opponent_discards_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponents: Vec<PlayerId> = g.state().apnap().filter(|&p| p != item.controller).collect();
    each_discards_cards(g, &opponents, item.object, 2)?;
    Ok(())
}

/// "If there are four or more card types among cards in your graveyard" (CR 207.2c): checked once, as the modes are
/// chosen (CR 700.2a).
fn has_delirium(q: &dyn Query, asker: &Asker) -> bool {
    delirium(q, asker.controller)
}

static MODAL: ModalDef = modes(&[
    mode(opponents_creatures_shrink, "Creatures your opponents control get -1/-1 until end of turn."),
    mode(each_opponent_discards_two, "Each opponent discards two cards."),
    mode(effects::each_opponent_loses_life_you_gain_life::<3>, "Each opponent loses 3 life and you gain 3 life."),
])
.up_to_if(has_delirium, 3);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::LETS_PLAY_A_GAME };
