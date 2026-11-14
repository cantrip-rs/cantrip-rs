use cantrip_api::*;
use cantrip_mechanics::{effects, lands, tokens, triggers};
use cantrip_oracle as oracle;

/// "Choose one" as the trigger goes on the stack (CR 603.3c, 700.2a): "Create a 1/1 white Soldier creature token" (CR 111.4) or
/// "Creatures you control get +1/+1 until end of turn" (CR 611.2a).
static MODAL: ModalDef = modes(&[
    mode(tokens::create_a_soldier, "Create a 1/1 white Soldier creature token."),
    mode(pump_creatures_you_control, "Creatures you control get +1/+1 until end of turn."),
]);

fn pump_creatures_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::pump_creatures_you_control(g, item, 1, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::another_nontoken_of_subtype_you_control_enters::<{ lands::sub(Subtype::Hero) }>),
        &MODAL,
        "Whenever another nontoken Hero you control enters, choose one \u{2014}\n\u{2022} Create a 1/1 white Soldier creature token.\n\u{2022} Creatures you control get +1/+1 until end of turn.",
    )],
    ..oracle::BLACK_PANTHER_VANGUARD
};

pub static EXTRA: &[&CardDef] = &[&tokens::SOLDIER];
