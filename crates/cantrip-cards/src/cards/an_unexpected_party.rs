use cantrip_api::*;
use cantrip_mechanics::chosen_type::{creatures_you_control_of_chosen_type, enters_choosing_a_creature_type};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Create X 2/2 red Dwarf creature tokens." X is the number chosen, not the mana actually spent (CR 107.3a), and the tokens
/// are created together, as one event (CR 111.1).
fn create_dwarves(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::DWARF, item.x as u32)?;
    Ok(())
}

/// The Adventure (CR 715): At the Door.
pub static AT_THE_DOOR: CardDef = CardDef { spell: spell(&[], create_dwarves), ..oracle::AT_THE_DOOR };

pub static CARD: CardDef = CardDef {
    enters: Some(enters_choosing_a_creature_type),
    statics: &[static_ability(Layer::PtModify, creatures_you_control_of_chosen_type, plus_pt::<2, 2>, "Creatures you control of the chosen type get +2/+2.")],
    adventure: Some(&AT_THE_DOOR),
    ..oracle::AN_UNEXPECTED_PARTY
};

pub static EXTRA: &[&CardDef] = &[&AT_THE_DOOR, &tokens::DWARF];
