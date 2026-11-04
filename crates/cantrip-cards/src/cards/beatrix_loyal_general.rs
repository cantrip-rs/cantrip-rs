use cantrip_api::*;
use cantrip_mechanics::{attach, targets, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of combat on your turn, you may attach any number of Equipment you control to target creature you
/// control." (CR 507.1, 701.3a).
fn attach_equipment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    attach::attach_any_number_of_equipment_to(g, item.controller, item.object, creature)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::BEGINNING_OF_YOUR_COMBAT,
        attach_equipment,
        "At the beginning of combat on your turn, you may attach any number of Equipment you control to target creature you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::BEATRIX_LOYAL_GENERAL
};
