use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// The Role under the controller, attached to the reflexive trigger's target; nothing when the target is no longer legal (CR 608.2b).
fn create_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::YOUNG_HERO_ROLE, item.controller, creature)?;
    Ok(())
}

const PAY: Question = Question::new("Pay", "pay {cost} to create a Young Hero Role token attached to target creature you control?", "", "Pay {cost}", "Don't pay");

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        |g, item| effects::may_pay_then_when_you_do(g, item, &CARD, cost("{1}"), &PAY),
        "When this creature enters, you may pay {1}. When you do, create a Young Hero Role token attached to target creature you control.",
    )],
    delayed: &[delayed_trigger(
        create_role,
        "When you do, create a Young Hero Role token attached to target creature you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::MERRY_BARDS
};

pub static EXTRA: &[&CardDef] = &[&tokens::YOUNG_HERO_ROLE];
