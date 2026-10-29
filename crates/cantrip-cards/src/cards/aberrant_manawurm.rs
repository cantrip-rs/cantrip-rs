use cantrip_api::*;
use cantrip_mechanics::{pump, triggers};
use cantrip_oracle as oracle;

/// "Whenever you cast an instant or sorcery spell, this creature gets +X/+0 until end of turn, where X is the amount of mana spent
/// to cast that spell" (CR 601.2h): read from the spell the ability triggered on, so it is right even if that spell has since been
/// countered; a spell cast without paying its mana cost had none spent. Nothing if this creature has left the battlefield.
fn grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(spell) = item.subject {
        let spent = g.obj(spell).spent.total();
        pump(g, item.object, item.object, spent as i16, 0);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::INSTANT_OR_SORCERY_YOU_CAST,
        grow,
        "Whenever you cast an instant or sorcery spell, this creature gets +X/+0 until end of turn, where X is the amount of mana spent to cast that spell.",
    )],
    ..oracle::ABERRANT_MANAWURM
};
