use cantrip_api::select::{Choice, LargestFn, SelectFn, Verdict, largest_distinct, set_part};
use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Controlled by different players" (CR 109.4, 115.1): no two of the creatures chosen have the same controller. Judged again as the spell
/// resolves, a creature that has left read as it last was (CR 608.2b, Run Away Together's ruling).
fn different_controllers(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let chosen: Vec<PlayerId> = choice.selected.iter().map(|&c| q.controller(c)).collect();
    let complete = chosen.iter().enumerate().all(|(i, p)| !chosen[..i].contains(p));
    let addable = Bits::of(choice.candidates.len(), |i| !chosen.contains(&q.controller(choice.candidates[i])));
    Verdict { complete, addable }
}

/// A largest set the rule allows: the first creature of each controller.
fn largest_different_controllers(q: &dyn Query, choice: &Choice, _: u32) -> Vec<ObjectId> {
    largest_distinct(choice, |c| q.controller(c))
}

const DIFFERENT_CONTROLLERS: Part =
    set_part(&(different_controllers as SelectFn), &(largest_different_controllers as LargestFn), 0, "controlled by different players");

/// "Return those creatures to their owners' hands": each one that is still a legal target (CR 608.2b), so when one of the two has become
/// illegal the other is still returned if it is still legal.
fn return_both(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = targets::legal_objects(g, item);
    g.return_to_hand(&creatures);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[TargetSpec { min: 2, max: 2, ..targets::CREATURE }.together(DIFFERENT_CONTROLLERS)], return_both),
    ..oracle::RUN_AWAY_TOGETHER
};
