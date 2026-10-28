use cantrip_api::*;
use cantrip_mechanics::attach::equipment_controlled;
use cantrip_mechanics::{choose, pump, select, targets};
use cantrip_oracle as oracle;

/// "Untap target creature you control. It gets +2/+2 until end of turn. If it's a Dwarf, you may attach an Equipment you
/// control to it." Dwarf is a creature type read through the spell's words (CR 612). The creature is judged as the spell
/// resolves (CR 608.2c); the choice of Equipment is made then, from those its controller controls, none being allowed, and
/// one that cannot be attached to the creature stays where it is (CR 701.3a-b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.untap(creature);
    g.end_sentence();
    pump(g, item.object, creature, 2, 2);
    g.end_sentence();
    let dwarf = g.words(item.object).subtype(Subtype::Dwarf);
    if !g.chars(creature).has_subtype(dwarf) {
        return Ok(());
    }
    let equipment = equipment_controlled(g, item.controller);
    let chosen = choose::select(g, item.controller, item.object, Purpose::Attach, equipment, select::cards(select::up_to(1)))?;
    if let Some(&attachment) = chosen.first() {
        g.attach(attachment, creature);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE_YOU_CONTROL], effect), ..oracle::VOW_TO_EREBOR };
