use cantrip_api::*;
use cantrip_mechanics::{statics, tokens, triggers::YOUR_END_STEP};
use cantrip_oracle as oracle;

/// "At the beginning of your end step, you lose 3 life and draw a card. Then if your life total is 0 or less, create ten
/// 1/1 red Officer creature tokens." The life check follows both instructions (CR 608.2c).
fn end_step(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    g.lose_life(you, 3);
    g.draw(you, 1);
    if g.state().player(you).life <= 0 {
        tokens::create(g, you, &tokens::OFFICER, 10)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    loss_rules: &[statics::you_cant_lose_from_life],
    abilities: &[triggered(
        YOUR_END_STEP,
        end_step,
        "At the beginning of your end step, you lose 3 life and draw a card. Then if your life total is 0 or less, create ten 1/1 red Officer creature tokens.",
    )],
    ..oracle::GEORGIOU_TERRAN_EMPEROR
};

pub static EXTRA: &[&CardDef] = &[&tokens::OFFICER];
