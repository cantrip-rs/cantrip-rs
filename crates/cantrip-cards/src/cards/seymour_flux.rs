use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_UPKEEP;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("Draw", "pay {cost} to draw a card and put a +1/+1 counter on Seymour Flux?", "", "Pay {cost}", "Don't pay");

/// "You may pay 1 life. If you do, draw a card and put a +1/+1 counter on Seymour Flux." (CR 118.12, 119.4, 122.1): the life is
/// paid as the ability resolves, only by a player with the life; both effects follow only when it was paid. The counter goes on
/// Seymour Flux itself, which is no new object once it has left the battlefield (CR 400.7).
fn pay_to_draw_and_grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, life(1), &ASK)? {
        g.draw(item.controller, 1);
        g.add_counters(item.object, CounterKind::PlusOne, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_UPKEEP,
        pay_to_draw_and_grow,
        "At the beginning of your upkeep, you may pay 1 life. If you do, draw a card and put a +1/+1 counter on Seymour Flux.",
    )],
    ..oracle::SEYMOUR_FLUX
};
