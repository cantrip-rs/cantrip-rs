use cantrip_api::*;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

const PAY: Question = Question::new("Pay", "pay {cost} to create a Sorcerer Role token attached to target creature you control?", "", "Pay {cost}", "Don't pay");

/// The Role under the controller, attached to the target; nothing when the target is no longer legal (CR 608.2b).
fn create_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::SORCERER_ROLE, item.controller, creature)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        |g, item| effects::may_pay_then_when_you_do(g, item, &CARD, cost("{1}"), &PAY),
        "At the beginning of combat on your turn, you may pay {1}. When you do, create a Sorcerer Role token attached to target creature you control. (Enchanted creature gets +1/+1 and has \"Whenever this creature attacks, scry 1.\")",
    )],
    delayed: &[delayed_trigger(
        create_role,
        "When you do, create a Sorcerer Role token attached to target creature you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::SPELLBOOK_VENDOR
};

pub static EXTRA: &[&CardDef] = &[&tokens::SORCERER_ROLE];
