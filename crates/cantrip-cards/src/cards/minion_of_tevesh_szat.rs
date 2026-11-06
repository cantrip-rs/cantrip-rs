use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

const PAY: Question = Question::new("Pay", "pay {cost}, or take 2 damage?", "", "Pay {cost}", "Take 2 damage");

/// The upkeep controller may pay {B}{B}; otherwise the Minion deals 2 damage to them (CR 118.12a, 120.3).
fn upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.may_pay(item.controller, item.object, item.object, cost("{B}{B}"), &PAY)? {
        g.damage(item.object, Target::Player(item.controller), 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            upkeep,
            "At the beginning of your upkeep, this creature deals 2 damage to you unless you pay {B}{B}.",
        ),
        ability(TAP, effects::target_creature_gets::<3, -2>, "{T}: Target creature gets +3/-2 until end of turn.")
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::MINION_OF_TEVESH_SZAT
};
