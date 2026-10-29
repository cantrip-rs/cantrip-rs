use cantrip_api::*;
use cantrip_mechanics::{spent, tokens};
use cantrip_oracle as oracle;

/// Converge (an ability word) — "Create two 0/0 green and blue Fractal creature tokens. Put X +1/+1 counters on each of them and you gain X life, where X
/// is the number of colors of mana spent to cast this spell" (CR 207.2c, 601.2h): colorless is not a color, a copy of the spell had
/// none spent, and an alternative or additional cost counts. The counters go on both tokens as one instruction.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = spent::colors_of_mana_spent(g, item.object);
    let made = tokens::create(g, item.controller, &tokens::FRACTAL, 2)?;
    let puts: Vec<(ObjectId, CounterKind, i16)> = made.iter().map(|&token| (token, CounterKind::PlusOne, x as i16)).collect();
    g.add_counters_together(&puts);
    g.gain_life(item.controller, x as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::SNARL_SONG };

pub static EXTRA: &[&CardDef] = &[&tokens::FRACTAL];
