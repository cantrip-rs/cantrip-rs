use cantrip_api::*;
use cantrip_mechanics::{damage_target, destroy_target, targets, tokens};
use cantrip_oracle as oracle;

/// Genesis Effect's three sentences resolve in order (CR 608.2c). The damage and destruction independently do nothing
/// if their respective target has become illegal (CR 608.2b); the token is still created if the spell has at least one
/// legal target. The land target must still be a land its controller controls.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 6);
    g.end_sentence();
    destroy_target(g, item, 1);
    g.end_sentence();
    tokens::create_by_rule(g, item.controller, &tokens::NEW_PLANET, 1, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef {
        targets: &[targets::CREATURE, targets::LAND_YOU_CONTROL],
        effect,
    }),
    ..oracle::GENESIS_EFFECT
};

pub static EXTRA: &[&CardDef] = &[&tokens::NEW_PLANET];
