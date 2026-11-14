use cantrip_api::*;
use cantrip_mechanics::{effects, select, targets, triggers};
use cantrip_oracle as oracle;

fn another_creature_or_enchantment(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |id| id != choice.source && (q.chars(id).is(CardType::Creature) || q.chars(id).is(CardType::Enchantment)))
}

const ANOTHER_CREATURE_OR_ENCHANTMENT: select::Part = select::part(&(another_creature_or_enchantment as SelectFn), 0, "");
const ANOTHER_CREATURE_OR_ENCHANTMENT_PERMANENT: select::Selection =
    select::Selection::of(select::up_to(1), &[ANOTHER_CREATURE_OR_ENCHANTMENT]).called("other creature or enchantment");

/// "You may sacrifice another creature or enchantment. When you do" (CR 701.21a, 603.12): the choice is made as the triggered
/// ability resolves; a reflexive trigger is created only if the permanent was actually sacrificed.
fn sacrifice_for_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::may_sacrifice(g, item.controller, item.object, ANOTHER_CREATURE_OR_ENCHANTMENT_PERMANENT)?.is_empty() {
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(g.card_id(&CARD), 0), Target::None);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, sacrifice_for_damage, "When this creature enters, you may sacrifice another creature or enchantment. When you do, this creature deals 2 damage to any target.")],
    delayed: &[delayed_trigger(effects::damage_first_target::<2>, "When you do, this creature deals 2 damage to any target.").targets(&[targets::ANY])],
    ..oracle::BOILERBILGES_RIPPER
};
