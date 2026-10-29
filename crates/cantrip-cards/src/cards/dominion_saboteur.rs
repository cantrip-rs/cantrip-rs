use cantrip_api::*;
use cantrip_oracle as oracle;
use cantrip_mechanics::copy;

const COPY_AS_ENTERS: Question = Question::new(
    "CopyAsEnters",
    "enter as a copy?",
    "You choose what it copies next. If you don't, it enters as itself",
    "Choose a copy",
    "Enter as itself",
);

/// "You may have this creature enter as a copy of any artifact or creature ... except it isn't legendary." If the copy is
/// chosen, its extra counters have the same kinds and amounts as that permanent's (CR 707.2, 707.9b, 614.12).
fn artifact_or_creature(chars: &Chars) -> bool {
    chars.is(CardType::Artifact) || chars.is(CardType::Creature)
}

fn not_legendary(_: &dyn Query, values: Copiable) -> Copiable {
    values.nonlegendary()
}

fn enter_as_copy_with_counters(g: &mut dyn Game, entry: &mut Entering) -> Result<(), Suspend> {
    let entering = entry.object;
    let candidates: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| id != entering && artifact_or_creature(&g.chars(id)))
        .collect();
    let Some(chosen) = copy::choose_copy_of_one_of(g, entry, &candidates, &COPY_AS_ENTERS)? else { return Ok(()) };
    entry.copy = Some(not_legendary(g, g.copiable_values(chosen)));
    for kind in CounterKind::ALL {
        let count = g.obj(chosen).counters(kind);
        if count > 0 {
            entry.counters[kind as usize] += count;
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { enters_as_copy: Some(enter_as_copy_with_counters), ..oracle::DOMINION_SABOTEUR };
