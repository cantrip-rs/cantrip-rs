use cantrip_api::*;
use cantrip_mechanics::targets::{self, filters};
use cantrip_mechanics::{counter_target, counter_target_ability};
use cantrip_oracle as oracle;

/// "Counter target activated ability, triggered ability, or noncreature spell" (CR 115.1, 605.3b): an ability on the stack
/// (a mana ability never is), or a spell that is not a creature spell.
fn target_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::activated_or_triggered_ability_filter(q, asker, t) || (targets::SPELL.filter)(q, asker, t) && matches!(t, Target::Object(id) if !q.chars(id).is(CardType::Creature))
}

/// A legendary creature, to sacrifice (CR 205.4a, 701.21).
fn legendary_creature(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    filters::creature(q, asker, id) && q.chars(id).supertypes.legendary
}

/// Counters the spell or the ability, whichever was targeted (CR 701.6, 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    match item.target(0) {
        Target::Ability(_) => {
            counter_target_ability(g, item, 0);
        }
        _ => {
            counter_target(g, item, 0);
        }
    }
    Ok(())
}

/// "As an additional cost to cast this spell, sacrifice a legendary creature or pay {2}" (CR 601.2b, 701.21a).
pub static CARD: CardDef = CardDef {
    spell: spell(&[target(target_filter)], effect),
    additional_choice: &[
        ExtraCost { text: "Sacrifice a legendary creature", cost: Cost::sacrifice(1, legendary_creature) },
        ExtraCost { text: "Pay {2}", cost: Cost { mana: mana("{2}"), ..Cost::NONE } },
    ],
    ..oracle::LOUISOIXS_SACRIFICE
};
