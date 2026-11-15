use cantrip_api::select::{Choice, LargestFn, SelectFn, Verdict, set_part};
use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets};
use cantrip_oracle as oracle;

/// "Controlled by the same opponent" (CR 109.4, 115.1): the creatures chosen all have one controller. Judged again as the spell
/// resolves, a creature that has left read as it last was (CR 608.2b).
fn same_controller(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let first = choice.selected.first().map(|&c| q.controller(c));
    let complete = choice.selected.iter().all(|&c| Some(q.controller(c)) == first);
    let addable = Bits::of(choice.candidates.len(), |i| first.is_none_or(|p| q.controller(choice.candidates[i]) == p));
    Verdict { complete, addable }
}

/// A largest set the rule allows: the creatures of the controller who has the most.
fn largest_same_controller(q: &dyn Query, choice: &Choice, _: u32) -> Vec<ObjectId> {
    let owned = |c: ObjectId| choice.candidates.iter().filter(|&&o| q.controller(o) == q.controller(c)).count() as u64;
    let best = choice.candidates.iter().copied().max_by_key(|&c| owned(c));
    best.map_or_else(Vec::new, |b| choice.candidates.iter().copied().filter(|&o| q.controller(o) == q.controller(b)).collect())
}

const SAME_CONTROLLER: Part = set_part(&(same_controller as SelectFn), &(largest_same_controller as LargestFn), 0, "controlled by the same opponent");

/// "That player chooses one of those creatures. Trial of Agony deals 5 damage to that creature, and the other can't block this turn"
/// (CR 608.2d, 509.1b): the creatures still legal as it resolves (CR 608.2b; ruling 2024-09-20: with one left, its controller
/// "chooses" it and it is dealt the damage). Their controller, the opponent, picks the one that takes the damage; the other
/// can't block until end of turn, an effect on the creature.
fn they_choose_one_it_takes_5_the_other_cant_block(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = targets::legal_objects(g, item);
    let Some(&first) = creatures.first() else { return Ok(()) };
    let chooser = g.controller(first);
    let rule = select::Selection::new(select::Count::Exactly(1)).called("creature");
    let hits = choose::select(g, chooser, item.object, Purpose::Choose, creatures.clone(), rule)?;
    let Some(&hit) = hits.first() else { return Ok(()) };
    g.damage(item.object, Target::Object(hit), 5);
    let others: Vec<ObjectId> = creatures.iter().copied().filter(|&c| c != hit).collect();
    cantrip_api::instruction(g, |g| {
        for &other in &others {
            g.add_effect(item.object, other, EffectKind::CantBlock, Duration::EndOfTurn);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[TargetSpec { min: 2, max: 2, ..targets::CREATURE_AN_OPPONENT_CONTROLS }.together(SAME_CONTROLLER)],
        they_choose_one_it_takes_5_the_other_cant_block,
    ),
    ..oracle::TRIAL_OF_AGONY
};
