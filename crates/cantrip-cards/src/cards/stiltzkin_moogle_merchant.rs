use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target opponent gains control of another target permanent you control. If they do, you draw a card." (CR 108.4, 608.2c):
/// no card is drawn if either target has become illegal, so the permanent did not change hands (CR 608.2b).
fn trade(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::target_opponent_gains_control_of_target_permanent(g, item) {
        g.end_sentence();
        effects::draw(g, item, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}, {T}"),
        trade,
        "{2}, {T}: Target opponent gains control of another target permanent you control. If they do, you draw a card.",
    )
    .targets(&[
        targets::OPPONENT.labelled("Opponent who gains control of the permanent"),
        targets::OTHER_PERMANENT_YOU_CONTROL.labelled("Permanent you control for that opponent to gain control of"),
    ])],
    ..oracle::STILTZKIN_MOOGLE_MERCHANT
};
