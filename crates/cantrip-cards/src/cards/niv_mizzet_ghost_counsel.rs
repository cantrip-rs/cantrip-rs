use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::YOU_GAIN_LIFE;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("PayLife", "pay {cost} to draw that many cards?", "", "Pay {cost}", "No");

/// "Whenever you gain life, you may pay that much life. If you do, draw that many cards." (CR 119.4, 118.12): "that much"
/// is the life that one gain gave, fixed as it triggered (`data[0]`), not the life total now; the life is a cost, paid
/// only by a player with it, and the cards are drawn only if it was paid.
fn pay_life_to_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let n = item.data[0];
    let cost = Cost { life: n, ..Cost::NONE };
    if g.may_pay(item.controller, item.object, item.object, cost, &ASK)? {
        g.draw(item.controller, n);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            YOU_GAIN_LIFE,
            pay_life_to_draw,
            "Whenever you gain life, you may pay that much life. If you do, draw that many cards.",
        ),
        ability(TAP, effects::each_opponent_loses_1_life_you_gain_1, "{T}: Each opponent loses 1 life and you gain 1 life."),
    ],
    ..oracle::NIV_MIZZET_GHOST_COUNSEL
};
