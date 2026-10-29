use cantrip_api::*;
use cantrip_mechanics::{blight, triggers};
use cantrip_oracle as oracle;

const BLIGHT_TWO: Question = Question::new("BlightTwo", "blight 2?", "", "Blight 2", "Don't");

/// "You may blight 2. If you don't, you lose 3 life" (CR 701.68a, 119.3): the life is lost when the player declines and also
/// when they can't blight (CR 701.68b), since either way they didn't.
fn blight_or_lose_3_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if blight::may_in_effect(g, item.controller, item.object, 2, &BLIGHT_TWO)?.is_none() {
        g.lose_life(item.controller, 3);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_FIRST_MAIN_PHASE,
        blight_or_lose_3_life,
        "At the beginning of your first main phase, you may blight 2. If you don't, you lose 3 life.",
    )],
    ..oracle::GUTSPLITTER_GANG
};
