use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Each of up to two target creatures" (CR 601.2c): one slot, up to two targets; each still legal gets a counter.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::BEGINNING_OF_YOUR_COMBAT, effects::plus_one_counter_on_each_target, "At the beginning of combat on your turn, put a +1/+1 counter on each of up to two target creatures.")
        .targets(&[targets::CREATURE.between(0, 2)])],
    ..oracle::AGENT_BISHOP_MAN_IN_BLACK
};
