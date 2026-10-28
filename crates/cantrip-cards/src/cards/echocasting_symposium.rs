use cantrip_api::*;
use cantrip_mechanics::{paradigm, targets};
use cantrip_oracle as oracle;

/// "Target player creates a token that's a copy of target creature you control." The token is made by, and controlled by, the
/// first target (CR 111.2, 707.2), not by the spell's controller; the copy is of the second target's copiable values as the
/// spell resolves (CR 707.2), so counters, damage, auras and effects are not copied. Each target that is no longer legal is
/// left out (CR 608.2b): with no player no one creates it, with no creature there is nothing to copy.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let (Some(player), Some(creature)) = (targets::player(g, item, 0), targets::object(g, item, 1)) else { return Ok(()) };
    g.create_token_copies(creature, player, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER, targets::CREATURE_YOU_CONTROL], effect),
    paradigm: Some(paradigm::SPELL_ABILITY),
    delayed: &[paradigm::TRIGGER],
    ..oracle::ECHOCASTING_SYMPOSIUM
};
