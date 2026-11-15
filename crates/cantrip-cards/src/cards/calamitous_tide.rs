use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Return up to two target creatures to their owners' hands. Draw two cards, then discard a card." (CR 608.2b-c): the targets
/// still legal return together, and the draw and discard follow whatever came back (even nothing), so the caster may discard a
/// creature just returned.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_each_target_to_hand(g, item)?;
    g.end_sentence();
    effects::draw_two_then_discard(g, item)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE.between(0, 2)], effect),
    ..oracle::CALAMITOUS_TIDE
};
