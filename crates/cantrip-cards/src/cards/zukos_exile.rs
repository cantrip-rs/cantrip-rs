use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "Exile target artifact, creature, or enchantment. Its controller creates a Clue token." (CR 701.13a, 701.7a, 608.2h): the controller
/// is read before the exile and is the one it last had, so the Clue is made for whoever controlled it whether or not it was a
/// token that ceased to exist; creating a Clue is not investigating (the 2025-10-02 ruling). If the target is no longer legal as
/// this resolves nothing happens (CR 608.2b).
fn exile_it_then_its_controller_creates_a_clue(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    let controller = g.controller(permanent);
    g.exile(permanent);
    tokens::create(g, controller, &tokens::CLUE, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(targets::artifact_creature_or_enchantment_filter)], exile_it_then_its_controller_creates_a_clue),
    ..oracle::ZUKOS_EXILE
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
