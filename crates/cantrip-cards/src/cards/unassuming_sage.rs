use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

const PAY: Question = Question::new("Pay", "pay {cost} to create a Sorcerer Role token attached to this creature?", "", "Pay {cost}", "Don't pay");

/// "You may pay {2}. If you do, create a Sorcerer Role token attached to it." The payment is chosen as the ability resolves
/// (CR 118.12, 608.2d). The Sage is "it" even if the Role could not enchant it (then no Role is created, CR 303.4i).
fn pay_for_sorcerer_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, cost("{2}"), &PAY)? {
        tokens::create_role(g, &tokens::SORCERER_ROLE, item.controller, item.object)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        pay_for_sorcerer_role,
        "When this creature enters, you may pay {2}. If you do, create a Sorcerer Role token attached to it. (Enchanted creature gets +1/+1 and has \"Whenever this creature attacks, scry 1.\")",
    )],
    ..oracle::UNASSUMING_SAGE
};

pub static EXTRA: &[&CardDef] = &[&tokens::SORCERER_ROLE];
