use cantrip_api::*;
use cantrip_mechanics::{creatures_you_control, damage_target, permanents_with_subtype_you_control, targets};
use cantrip_oracle as oracle;

/// "Slash of Light deals damage equal to the number of creatures you control plus the number of Equipment you control to target
/// creature." (CR 120.3, 301.5): both counted as it resolves (CR 608.2h, the ruling). An Equipment that is also a creature counts
/// once as each.
fn slash(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = creatures_you_control(g, item.controller);
    let equipment = permanents_with_subtype_you_control(g, item.controller, Subtype::Equipment);
    damage_target(g, item, 0, (creatures + equipment) as u16);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], slash), ..oracle::SLASH_OF_LIGHT };
