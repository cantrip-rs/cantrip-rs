use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Create a 0/0 green and blue Fractal creature token. Put X +1/+1 counters on it. Surveil 2." The counters are put on after the
/// token is made, so with X of 0 it is a 0/0 for a moment and the state-based actions put it into the graveyard (CR 704.5f); surveil
/// is a separate instruction (CR 608.2c).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let made = tokens::create(g, item.controller, &tokens::FRACTAL, 1)?;
    let puts: Vec<(ObjectId, CounterKind, i16)> = made.iter().map(|&token| (token, CounterKind::PlusOne, item.x as i16)).collect();
    g.add_counters_together(&puts);
    g.end_sentence();
    effects::surveil(g, item, 2)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::WILD_HYPOTHESIS };

pub static EXTRA: &[&CardDef] = &[&tokens::FRACTAL];
