use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_mechanics::{increment, tokens};
use cantrip_oracle as oracle;

/// The intervening "if it had one or more counters on it" (CR 603.4, 603.10a): the creature as it last was on the battlefield,
/// counters of every kind.
fn had_counters(q: &dyn Query, asker: &Asker) -> bool {
    CounterKind::ALL.iter().any(|&kind| q.obj(asker.source).counters(kind) > 0)
}

/// "Create a 0/0 green and blue Fractal creature token, then put this creature's counters on that token": two instructions
/// (CR 608.2c). The counters are those it had as it last existed (CR 608.2h), each kind in its exact number, put on every
/// token that was made (a token doubler makes two), all together as one event.
fn pass_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let counters = g.obj(item.object).counter_array();
    let made = tokens::create(g, item.controller, &tokens::FRACTAL, 1)?;
    g.end_sentence();
    let puts: Vec<(ObjectId, CounterKind, i16)> =
        made.iter().flat_map(|&token| CounterKind::ALL.into_iter().map(move |kind| (token, kind, counters[kind as usize]))).collect();
    g.add_counters_together(&puts);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        increment::TRIGGER,
        triggered(
            THIS_DIES,
            pass_counters,
            "When this creature dies, if it had one or more counters on it, create a 0/0 green and blue Fractal creature token, then put this creature's counters on that token.",
        )
        .condition(had_counters),
    ],
    ..oracle::AMBITIOUS_AUGMENTER
};

pub static EXTRA: &[&CardDef] = &[&tokens::FRACTAL];
