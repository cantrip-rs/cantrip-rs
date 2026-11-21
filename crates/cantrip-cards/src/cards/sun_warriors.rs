use cantrip_api::*;
use cantrip_mechanics::{creatures_you_control, firebending, tokens};
use cantrip_oracle as oracle;

/// "Firebending X, where X is the number of creatures you control": counted as the ability resolves (CR 608.2h), this creature
/// among them if it is still a creature on the battlefield.
fn firebend_creatures_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = creatures_you_control(g, item.controller) as i64;
    firebending::add_x(g, item.controller, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        firebending::trigger_x(firebend_creatures_you_control, "Firebending X, where X is the number of creatures you control. (Whenever this creature attacks, add X {R}. This mana lasts until end of combat.)"),
        ability(cost("{5}"), tokens::create_an_ally, "{5}: Create a 1/1 white Ally creature token."),
    ],
    ..oracle::SUN_WARRIORS
};

pub static EXTRA: &[&CardDef] = &[&tokens::ALLY];
