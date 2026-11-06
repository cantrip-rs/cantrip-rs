use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

const PAY: Question = Question::new(
    "Pay",
    "pay {cost} to stop this ability's controller gaining control of {subject}?",
    "",
    "Pay {cost}",
    "Don't pay",
);

/// An opponent may pay {2}; otherwise gain control for as long as this creature remains (CR 118.12a, 611.2b).
fn gain_control_unless_paid(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(artifact) = targets::object(g, item, 0) else { return Ok(()) };
    let opponents: Vec<PlayerId> = g.state().apnap().filter(|&p| p != item.controller).collect();
    for opponent in opponents {
        if g.may_pay(opponent, item.object, artifact, cost("{2}"), &PAY)? {
            return Ok(());
        }
    }
    instruction(g, |g| {
        g.gain_control(item.object, artifact, item.controller, Duration::WhileSourceOnBattlefield);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}{G}").and(TAP),
        gain_control_unless_paid,
        "{2}{G}, {T}: Unless an opponent pays {2}, gain control of target artifact for as long as this creature remains on the battlefield.",
    ).targets(&[targets::ARTIFACT])],
    ..oracle::SCARWOOD_BANDITS
};
