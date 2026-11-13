use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Whenever this creature attacks, it deals 3 damage to you unless you pay {2}." (CR 608.2, 120.3, 118.12).
fn deals_3_damage_unless_pay(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.may_pay(item.controller, item.object, item.object, cost("{2}"), &PAY_QUESTION)? {
        g.damage(item.object, Target::Player(item.controller), 3);
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new("Pay", "pay {cost}, or this creature deals 3 damage to you?", "", "Pay {cost}", "Take 3 damage");

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ATTACKS,
            deals_3_damage_unless_pay,
            "Whenever this creature attacks, it deals 3 damage to you unless you pay {2}.",
        ),
    ],
    ..oracle::HASRAN_OGRESS
};
