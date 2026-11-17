use cantrip_api::*;
use cantrip_mechanics::{effects, targets, vivid};
use cantrip_oracle as oracle;

/// "Tap up to two target creatures. Put a stun counter on each of them. Draw a card." The creatures still legal as it resolves are
/// tapped and each gets its counter (`effects::tap_and_stun_each_target`); with every target illegal the spell does not resolve and
/// nothing is drawn (the rulings), while with none chosen it resolves and draws.
fn tap_stun_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::tap_and_stun_each_target(g, item)?;
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[vivid::cheaper_per_color],
    spell: spell(&[targets::CREATURE.between(0, 2)], tap_stun_and_draw),
    ..oracle::RIME_CHILL
};
