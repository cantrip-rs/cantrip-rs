use cantrip_api::*;
use cantrip_mechanics::{damage_target, effects, select, targets, triggers};
use cantrip_oracle as oracle;

fn another_nonland(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |id| id != choice.source && !q.chars(id).is(CardType::Land))
}

const ANOTHER_NONLAND: select::Part = select::part(&(another_nonland as SelectFn), 0, "");
const ANOTHER_NONLAND_PERMANENT: select::Selection =
    select::Selection::of(select::up_to(1), &[ANOTHER_NONLAND]).called("other nonland permanent");

/// "You may sacrifice another nonland permanent. When you do" (CR 701.21a, 603.12): the choice is made as the triggered
/// ability resolves; a reflexive trigger is created only if the permanent was actually sacrificed.
fn sacrifice_for_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::may_sacrifice(g, item.controller, item.object, ANOTHER_NONLAND_PERMANENT)?.is_empty() {
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(g.card_id(&CARD), 0), Target::None);
    }
    Ok(())
}

/// "This creature deals 2 damage to any target" (CR 120.2b, 608.2b).
fn deal_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, sacrifice_for_damage, "When this creature enters, you may sacrifice another nonland permanent. When you do, this creature deals 2 damage to any target.")],
    delayed: &[delayed_trigger(deal_two, "When you do, this creature deals 2 damage to any target.").targets(&[targets::ANY])],
    ..oracle::DAHAR_MASTER
};
