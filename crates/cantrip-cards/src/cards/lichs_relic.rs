use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, statics::plus_pt, targets};
use cantrip_oracle as oracle;

/// What the payment asks: paying {2} is what makes the destroying ability happen (CR 603.12); the targets are chosen once it is
/// paid.
const ASK: Question = Question::new(
    "DestroyCreatureOrPlaneswalker",
    "pay {cost} to destroy up to one target creature or planeswalker each opponent controls?",
    "When you do, for each opponent, destroy up to one target creature or planeswalker that player controls",
    "Pay {cost}",
    "Don't pay",
);

/// The slot for one opponent of "for each opponent, destroy up to one target creature or planeswalker that player controls"
/// (CR 601.2c, 115.1): a creature or planeswalker that opponent controls. The core makes a slot of its own for each
/// opponent (`TargetSpec::EACH_OPPONENT`) and says which one this is.
fn creature_or_planeswalker_the_slots_opponent_controls(q: &dyn Query, _: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    targets::controlled_by_slot_opponent(q, id) && (q.chars(id).is(CardType::Creature) || q.chars(id).is(CardType::Planeswalker))
}

const FOR_EACH_OPPONENT: TargetSpec = target(creature_or_planeswalker_the_slots_opponent_controls)
    .between(0, TargetSpec::EACH_OPPONENT)
    .labelled("Creature or planeswalker {player} controls to destroy");

/// "... destroy up to one target creature or planeswalker that player controls" for each opponent: the targets still legal
/// when it resolves are destroyed (CR 701.8, 608.2b), a target that has become illegal left alone.
fn destroy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects = targets::legal_objects(g, item);
    g.destroy_all(&objects);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| effects::may_pay_then_when_you_do(g, item, &CARD, cost("{2}"), &ASK), "When this Equipment enters, you may pay {2}. When you do, for each opponent, destroy up to one target creature or planeswalker that player controls."),
        equip(mana("{2}"), "Equip {2}"),
    ],
    delayed: &[delayed_trigger(destroy, "When you do, for each opponent, destroy up to one target creature or planeswalker that player controls.")
        .targets(&[FOR_EACH_OPPONENT])],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<2, 1>, "Equipped creature gets +2/+1.")],
    ..oracle::LICHS_RELIC
};
