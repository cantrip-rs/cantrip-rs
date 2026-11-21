use cantrip_api::*;
use cantrip_mechanics::combat::create_attacking_of_your_choice;
use cantrip_mechanics::{statics, tokens, triggers};
use cantrip_oracle as oracle;

/// "Create a 1/1 white Ally creature token that's tapped and attacking" (CR 508.4): its controller chooses what it attacks as
/// it enters, asked only when there is a choice. It was never declared as an attacker, so Suki does not trigger for it.
fn ally_tapped_and_attacking(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let ally = g.card_id(&tokens::ALLY);
    create_attacking_of_your_choice(g, item.controller, item.object, ally, 1, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // "Suki's power is equal to the number of creatures you control." (CR 604.3, 613.4a): a characteristic-defining ability that
    // counts Suki herself while she is a creature on the battlefield.
    statics: &[characteristic_defining(Layer::PtDefine, statics::power_is_creatures_you_control, "Suki's power is equal to the number of creatures you control.").lost_pt(0, 4)],
    abilities: &[triggered(triggers::THIS_ATTACKS, ally_tapped_and_attacking, "Whenever Suki attacks, create a 1/1 white Ally creature token that's tapped and attacking.")],
    ..oracle::SUKI_KYOSHI_WARRIOR
};

pub static EXTRA: &[&CardDef] = &[&tokens::ALLY];
