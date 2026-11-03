use cantrip_api::*;
use cantrip_mechanics::copy;
use cantrip_oracle as oracle;

/// "You may have this creature enter as a copy of a creature or planeswalker you control, except it enters with an additional +1/+1 counter on it if it's a creature, it enters with an additional loyalty counter on it if it's a planeswalker, and it isn't legendary." (CR 608.2, 701.5a).
fn have_creature_enter_as_copy_enters(
    g: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    let asker = &Asker::new(entering.object, entering.controller);
    const ASK_0: Question = Question::new(
        "CopyAsEnters",
        "enter as a copy?",
        "You choose what it copies next. If you don't, it enters as itself",
        "Choose a copy",
        "Enter as itself",
    );
    let candidates_0: Vec<ObjectId> = ({
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                (q.chars(id).is(CardType::Creature)
                    || q.chars(id).is(CardType::Planeswalker))
                    && Target::Object(asker.source)
                        .object()
                        .map(|o: ObjectId| q.controller(o)) == Some(q.controller(id))
            })
            .collect::<Vec<ObjectId>>()
    })
        .into_iter()
        .filter(|&id| id != entering.object)
        .collect();
    let Some(chosen_0) = copy::choose_copy_of_one_of(g, entering, &candidates_0, &ASK_0)? else { return Ok(()) };
    entering.copy = {
        let q: &dyn Query = g;
        Some(chosen_0)
            .map(|original: ObjectId| q.copiable_values(original).nonlegendary())
    };
    let picked_0 = vec![chosen_0];
    if {
        let q: &dyn Query = g;
        (picked_0
            .clone()
            .into_iter()
            .filter(|_| {
                entering
                    .copy
                    .as_ref()
                    .is_some_and(|c| {
                        c.extra_types.has(CardType::Creature)
                            || !c.types_stated() && q.card(c.card).is(CardType::Creature)
                    })
            })
            .collect::<Vec<ObjectId>>()
            .len() as i64) >= 1
    } {
        entering.counters[CounterKind::PlusOne as usize] = entering
            .counters[CounterKind::PlusOne as usize]
            .saturating_add(1);
    }
    if {
        let q: &dyn Query = g;
        (picked_0
            .clone()
            .into_iter()
            .filter(|_| {
                entering
                    .copy
                    .as_ref()
                    .is_some_and(|c| {
                        c.extra_types.has(CardType::Planeswalker)
                            || !c.types_stated()
                                && q.card(c.card).is(CardType::Planeswalker)
                    })
            })
            .collect::<Vec<ObjectId>>()
            .len() as i64) >= 1
    } {
        entering.counters[CounterKind::Loyalty as usize] = entering
            .counters[CounterKind::Loyalty as usize]
            .saturating_add(1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters_as_copy: Some(have_creature_enter_as_copy_enters),
    ..oracle::SPARK_DOUBLE
};
