use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever you face a dilemma, you may pay {B}. If you do, return this card from your graveyard to your hand."
fn may_pay_to_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_return_this_to_hand(g, item, of_mana(mana("{B}")))
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
