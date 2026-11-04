use cantrip_api::*;
use cantrip_mechanics::{effects, mana, triggers};
use cantrip_oracle as oracle;

/// "Whenever you cast a noncreature spell, you may pay {B}. If you do, return this card from your graveyard to your hand."
fn pay_to_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_return_this_to_hand(g, item, cost("{B}"))
}

pub static CARD: CardDef = CardDef {
    // "This creature enters tapped." (CR 614.1d)
    enters: Some(mana::enters_tapped),
    abilities: &[triggered(
        triggers::NONCREATURE_SPELL_YOU_CAST,
        pay_to_return,
        "Whenever you cast a noncreature spell, you may pay {B}. If you do, return this card from your graveyard to your hand.",
    )
    // It returns this card from the graveyard, so it functions only there (CR 113.6m).
    .functions_in(IN_A_GRAVEYARD)],
    ..oracle::SHAMBLING_CIETH
};
