use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// Each player may pay in turn order; if none does, the controller sacrifices this (CR 118.12a, 101.4).
fn upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let players: Vec<PlayerId> = g.state().apnap().collect();
    for player in players {
        if g.may_pay(player, item.object, item.object, cost("{3}"), &PAY_QUESTION)? {
            return Ok(());
        }
    }
    g.sacrifice(item.controller, &[item.object]);
    Ok(())
}

static PAY_QUESTION: Question = Question::new("Pay", "pay {cost}?", "", "Pay {cost}", "Decline");

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::exile_first_target_with_this, "When this enchantment enters, exile target creature.")
            .targets(&[targets::CREATURE]),
        triggered(triggers::YOUR_UPKEEP, upkeep, "At the beginning of your upkeep, sacrifice this enchantment unless any player pays {3}."),
        triggered(triggers::THIS_LEAVES, effects::return_exiled_to_battlefield, "When this enchantment leaves the battlefield, return the exiled card to the battlefield under its owner's control."),
    ],
    ..oracle::ICY_PRISON
};
