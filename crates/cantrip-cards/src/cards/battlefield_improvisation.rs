use cantrip_api::*;
use cantrip_mechanics::attach::attach_any_number_of_equipment_to;
use cantrip_mechanics::{pump, targets};
use cantrip_oracle as oracle;

/// Target creature gets +2/+2 until end of turn. If that creature is attacking, you may attach any number of Equipment you
/// control to it.
///
/// "Any number" is a choice of Equipment, none included (CR 701.3a): made as the spell resolves, from the Equipment its
/// controller controls then, attached one after another. The creature is read as it is as the spell resolves, so one that
/// has been removed from combat is not "attacking" (CR 506.4); an Equipment that cannot be attached to it stays where it
/// is (CR 701.3b), as does one already attached to it.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    pump(g, item.object, creature, 2, 2);
    if !g.is_attacking(creature) {
        return Ok(());
    }
    attach_any_number_of_equipment_to(g, item.controller, item.object, creature)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    ..oracle::BATTLEFIELD_IMPROVISATION
};
