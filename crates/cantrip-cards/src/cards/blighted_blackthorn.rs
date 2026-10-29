use cantrip_api::*;
use cantrip_mechanics::{blight, triggers};
use cantrip_oracle as oracle;

const BLIGHT_TWO: Question = Question::new("BlightTwo", "blight 2?", "", "Blight 2", "Don't");

/// "You may blight 2. If you do, you draw a card and lose 1 life" (CR 701.68a, 121.1, 119.3): the card and the life only if the
/// counters were put on a creature.
fn blight_then_draw_and_lose_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if blight::may_in_effect(g, item.controller, item.object, 2, &BLIGHT_TWO)?.is_some() {
        g.draw(item.controller, 1);
        g.lose_life(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS_OR_ATTACKS,
        blight_then_draw_and_lose_life,
        "Whenever this creature enters or attacks, you may blight 2. If you do, you draw a card and lose 1 life.",
    )],
    ..oracle::BLIGHTED_BLACKTHORN
};
