use cantrip_api::*;
use cantrip_mechanics::{blight, effects, triggers};
use cantrip_oracle as oracle;

const BLIGHT_ONE: Question = Question::new("BlightOne", "blight 1?", "", "Blight 1", "Don't");

/// "You may blight 1. If you do, each opponent discards a card" (CR 701.68a, 701.9).
fn blight_then_discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if blight::may_in_effect(g, item.controller, item.object, 1, &BLIGHT_ONE)?.is_some() {
        effects::each_opponent_discards_a_card(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, blight_then_discard, "When this creature enters, you may blight 1. If you do, each opponent discards a card.")],
    ..oracle::DREAM_SEIZER
};
