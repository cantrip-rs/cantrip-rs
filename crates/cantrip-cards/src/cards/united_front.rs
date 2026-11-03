use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Create X 1/1 white Ally creature tokens, then put a +1/+1 counter on each creature you control." X is the number chosen, not
/// the mana actually spent (CR 107.3a); the tokens are created together, as one event (CR 111.1), and then the counters are put on
/// the creatures the controller has by then, the new tokens too (CR 608.2c).
fn tokens_then_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::ALLY, item.x as u32)?;
    effects::plus_one_counter_on_each_creature_you_control(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], tokens_then_counters), ..oracle::UNITED_FRONT };

pub static EXTRA: &[&CardDef] = &[&tokens::ALLY];
