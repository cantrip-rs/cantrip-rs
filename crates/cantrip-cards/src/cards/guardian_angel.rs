use cantrip_api::*;
use cantrip_mechanics::{prevention, targets};
use cantrip_oracle as oracle;

/// "Prevent the next X damage that would be dealt to any target this turn. Until end of turn, you may pay {1} any time you
/// could cast an instant." A shield of X on the target (CR 615.7), then (CR 608.2c) an effect that lets the spell's
/// controller take an action at a later time (CR 116.2c): paying {1}, a special action they may take any time they have
/// priority until the turn ends, as often as they pay. The action is about the permanent or player the spell targeted, as
/// it was then, whatever has become of it since. A target no longer legal stops the whole spell (CR 608.2b).
fn resolve(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    prevention::next_damage_to_target(g, item, 0, 0, u32::from(item.x));
    g.end_sentence();
    let about = g.state().target(item, 0);
    let card = g.obj(item.object).card;
    g.add_effect(item.object, item.object, EffectKind::PlayerSpecialAction { ability: AbilityRef::delayed(card, 0), about }, Duration::EndOfTurn);
    Ok(())
}

/// "If you do, prevent the next 1 damage that would be dealt to that permanent or player this turn" (CR 615.7): what paying
/// does, done at once (CR 116.2c). "That permanent" is the object the spell targeted: once it has left the battlefield it
/// is a new object (CR 400.7), and the shield has nothing to be on. Nothing targets it now, so what it has become since
/// (protection from white) does not matter.
fn shield_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let about = g.state().target(item, 0);
    prevention::this_turn(g, item, about, 1, Shield::Next(1));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ANY], resolve),
    delayed: &[ability(
        cost("{1}"),
        shield_one,
        "Until end of turn, you may pay {1} any time you could cast an instant. If you do, prevent the next 1 damage that would be dealt to that permanent or player this turn.",
    )
    .special_action()],
    granted_preventions: &[
        prevention::ANY_DAMAGE.text("Prevent the next X damage that would be dealt to any target this turn."),
        prevention::ANY_DAMAGE.text("Prevent the next 1 damage that would be dealt to that permanent or player this turn."),
    ],
    ..oracle::GUARDIAN_ANGEL
};
