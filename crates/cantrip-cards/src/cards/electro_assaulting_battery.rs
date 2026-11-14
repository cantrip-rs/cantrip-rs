use cantrip_api::*;
use cantrip_mechanics::triggers::{INSTANT_OR_SORCERY_YOU_CAST, THIS_LEAVES};
use cantrip_mechanics::{damage_target, effects, targets};
use cantrip_oracle as oracle;

const PAY_X: Question = Question::new(
    "DealXDamage",
    "pay {X}?",
    "When you do, Electro deals X damage to target player",
    "Pay {cost}",
    "Don't pay",
)
.no_default();

/// "You don't lose unspent red mana as steps and phases end" (CR 106.4, 500.5): its controller's red mana, restricted or not
/// (the rulings), stays in their pool while Electro is on the battlefield. Once he leaves, it is lost as the step or phase
/// that is under way ends.
fn keeps_red(_: &dyn Query, asker: &Asker, player: PlayerId, mana: Mana) -> bool {
    player == asker.controller && mana == Mana::R
}

/// "Add {R}": a triggered ability that adds mana and doesn't trigger from a mana ability is not a mana ability. It uses the
/// stack (CR 605.1b).
fn add_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::R, 1);
    Ok(())
}

/// "You may pay {X}. When you do, ..." The ability has no target. X is chosen as it resolves and paid (CR 107.3f), 0
/// included. Paying is doing it, and the reflexive ability (CR 603.12) triggers carrying X, with its target chosen as it goes
/// on the stack (the rulings).
fn pay_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_x_then_when_you_do(g, item, &CARD, &PAY_X, Target::None)
}

/// "He deals X damage to target player" (CR 120.3): Electro, as he last existed on the battlefield (CR 113.7a), deals the X
/// that was paid.
fn deal_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, u32::from(item.x));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(INSTANT_OR_SORCERY_YOU_CAST, add_red, "Whenever you cast an instant or sorcery spell, add {R}.").produces_mana(),
        triggered(THIS_LEAVES, pay_x, "When Electro leaves the battlefield, you may pay {X}. When you do, he deals X damage to target player."),
    ],
    delayed: &[delayed_trigger(deal_x, "When you do, he deals X damage to target player.").targets(&[targets::PLAYER])],
    mana_kept: &[keeps_red],
    ..oracle::ELECTRO_ASSAULTING_BATTERY
};
