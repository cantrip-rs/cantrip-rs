use cantrip_api::*;
use cantrip_mechanics::{effects, select, targets, triggers};
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
    effects::may_sacrifice_when_you_do(g, item, ANOTHER_NONLAND_PERMANENT)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, sacrifice_for_damage, "When this creature enters, you may sacrifice another nonland permanent. When you do, this creature deals 2 damage to any target.")],
    delayed: &[delayed_trigger(effects::damage_first_target::<2>, "When you do, this creature deals 2 damage to any target.").targets(&[targets::ANY])],
    ..oracle::DAHAR_MASTER
};
