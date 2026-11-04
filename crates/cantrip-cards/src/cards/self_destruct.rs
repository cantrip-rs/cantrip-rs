use cantrip_api::*;
use cantrip_mechanics::{fight, targets};
use cantrip_oracle as oracle;

/// "Target creature you control deals X damage to any other target and X damage to itself, where X is its power." (CR 120.3,
/// 608.2b, 608.2h): X is its power once, as this resolves. The creature is the source, so an illegal creature deals nothing at
/// all; a creature that is legal still deals X to itself when the other target is not (the rulings). Both are dealt together.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let x = fight::damage_equal_to_power(g, creature);
    let mut hits = vec![(creature, Target::Object(creature), x)];
    if g.target_legal(item, 1) {
        hits.push((creature, g.state().target(item, 1), x));
    }
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::ANY_TARGET_OTHER_THAN_THOSE_CHOSEN], effect),
    ..oracle::SELF_DESTRUCT
};
