use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

const PAY: Question = Question::new(
    "Pay",
    "pay {B} to return Gabriel Lorca from your graveyard to your hand?",
    "Return it to your hand",
    "Pay {B}",
    "Don't pay",
);

/// The choice of modes is a single dilemma regardless of how many modes were chosen (CR 700.2b).
fn may_pay_to_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(
        item.controller,
        item.object,
        item.object,
        of_mana(mana("{B}")),
        &PAY,
    )? {
        g.return_to_hand(&[item.object]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_FACE_A_DILEMMA,
        may_pay_to_return,
        "Whenever you face a dilemma, you may pay {B}. If you do, return this card from your graveyard to your hand.",
    )
    .functions_in(IN_A_GRAVEYARD)],
    ..oracle::GABRIEL_LORCA_MIRROR_MENACE
};
