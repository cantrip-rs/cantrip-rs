use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Create a 2/1 black Villain creature token with menace, then creatures you control get +1/+0 until end of turn." The token is
/// a creature you control as the pump applies, so it gets it too (CR 608.2c: "then" ends the first instruction); no creature
/// that enters later does (CR 611.2c).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::VILLAIN_2_1_BLACK_MENACE, 1)?;
    g.end_sentence();
    effects::pump_creatures_you_control(g, item, 1, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::HIRE_A_CREW };

pub static EXTRA: &[&CardDef] = &[&tokens::VILLAIN_2_1_BLACK_MENACE];
