use cantrip_api::*;
use cantrip_mechanics::combat::EXCEPT_BY_HASTE;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// What the payment asks: paying {1} is what makes the reflexive ability trigger (CR 603.12).
const ASK: Question = Question::new(
    "PayForBlockRestriction",
    "pay {cost} so target creature with haste can't be blocked this turn except by creatures with haste?",
    "When you do, target creature with haste can't be blocked this turn except by creatures with haste",
    "Pay {cost}",
    "Don't pay",
);

/// "Whenever you cast a noncreature spell, you may pay {1}. When you do, ..." (CR 118.12, 603.12): paying is doing it, and the
/// reflexive ability triggers then, its target chosen as it is put on the stack; with no creature with haste it is simply
/// removed (CR 603.3d), the {1} staying spent. Declining pays nothing and triggers nothing.
fn pay(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, cost("{1}"), &ASK)? {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    }
    Ok(())
}

/// "Target creature with haste" (CR 115.1, 702.10), read from its computed characteristics as it is targeted.
fn creature_with_haste(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id) && q.chars(id).is(CardType::Creature) && q.chars(id).has(Keyword::Haste))
}

/// "... can't be blocked this turn except by creatures with haste": a resolved effect on the creature, not an ability it gains
/// (CR 113.1a, 611.2c), which ends if it leaves the battlefield (CR 400.7). A creature that has become illegal as a target is
/// left alone (CR 608.2b).
fn cant_be_blocked_except_by_haste(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_effect(item.object, creature, EffectKind::BlockRestriction { card: item.ability.card, index: 0 }, Duration::EndOfTurn);
    Ok(())
}

/// Haste is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::NONCREATURE_SPELL_YOU_CAST,
        pay,
        "Whenever you cast a noncreature spell, you may pay {1}. When you do, target creature with haste can't be blocked this turn except by creatures with haste.",
    )],
    delayed: &[delayed_trigger(
        cant_be_blocked_except_by_haste,
        "When you do, target creature with haste can't be blocked this turn except by creatures with haste.",
    )
    .targets(&[target(creature_with_haste)])],
    lasting_block_rules: &[EXCEPT_BY_HASTE],
    ..oracle::SPEED_YOUNG_AVENGER
};
