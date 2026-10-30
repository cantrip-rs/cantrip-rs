use cantrip_api::*;
use cantrip_mechanics::{blight, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

const PAY: Question = Question::new("Pay", "pay {cost} so that you don't blight 2?", "", "Pay {cost}", "Blight 2");

/// "You may pay {2}. If you don't, blight 2." The payment is chosen as the ability resolves; a player who declines (or can't
/// pay) blights 2 unless they control no creature to blight (CR 701.68b).
fn pay_or_blight(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.may_pay(item.controller, item.object, item.object, cost("{2}"), &PAY)? {
        blight::in_effect(g, item.controller, item.object, 2)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        pay_or_blight,
        "When this creature enters, you may pay {2}. If you don't, blight 2. (To blight 2, put two -1/-1 counters on a creature you control.)",
    )],
    ..oracle::CHAOS_SPEWER
};
