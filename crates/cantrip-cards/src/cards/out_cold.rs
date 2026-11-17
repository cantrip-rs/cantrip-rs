use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Tap up to two target creatures and put a stun counter on each of them. Investigate." (CR 701.26a, 122.1d, 701.16a): the targets
/// still legal as it resolves are tapped and each gets its counter (`effects::tap_and_stun_each_target`), then you investigate
/// (CR 608.2c). With every target illegal the spell does not resolve, so there is no Clue (the rulings); with none
/// chosen it resolves and investigates. "This spell can't be countered" is the core keyword from the content JSON.
fn tap_stun_and_investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::tap_and_stun_each_target(g, item)?;
    tokens::investigate(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE.between(0, 2)], tap_stun_and_investigate),
    ..oracle::OUT_COLD
};
