use cantrip_api::*;
use cantrip_mechanics::{effects::gain_control_untap_haste, targets, tokens};
use cantrip_oracle as oracle;

/// "Gain control of target creature until end of turn. Untap that creature. It gains haste until end of turn. Create a Treasure
/// token." (CR 111.10a, 613.1b, 611.2a): the Treasure is made even when the creature was already under its controller's control
/// (the Act of Treason effect does not need a change of controller); nothing at all if the target is no longer legal (CR 608.2b).
fn involuntary_employment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    gain_control_untap_haste(g, item);
    tokens::create_treasure(g, item.controller)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], involuntary_employment), ..oracle::INVOLUNTARY_EMPLOYMENT };

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
