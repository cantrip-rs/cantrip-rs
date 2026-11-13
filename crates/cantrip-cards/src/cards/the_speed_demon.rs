use cantrip_api::*;
use cantrip_mechanics::start_your_engines::speed;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_oracle as oracle;

/// "You draw X cards and lose X life, where X is your speed" (CR 121.1, 119.3, 702.179f): X is read as the ability resolves, 0 for
/// a player with no speed.
fn draw_and_lose_speed(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = speed(g.state(), item.controller);
    g.draw(item.controller, u32::from(x));
    g.lose_life(item.controller, i32::from(x));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[triggered(YOUR_END_STEP, draw_and_lose_speed, "At the beginning of your end step, you draw X cards and lose X life, where X is your speed.")],
    ..oracle::THE_SPEED_DEMON
};
