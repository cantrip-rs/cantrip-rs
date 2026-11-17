use cantrip_api::*;
use cantrip_mechanics::{effects, flurry};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on this creature. It can't be blocked this turn." (CR 122.1, 509.1b, 611.2), in the order printed (CR 608.2c).
fn grow_and_become_unblockable(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counter_on_this(g, item)?;
    g.end_sentence();
    effects::this_cant_be_blocked_this_turn(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[flurry::trigger("Whenever you cast your second spell each turn, put a +1/+1 counter on this creature. It can't be blocked this turn.", grow_and_become_unblockable)],
    ..oracle::RAZZLE_DAZZLER
};
