use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Gain control of target creature until end of turn. Untap that creature. It gains haste until end of turn. Create a Wicked Role
/// token attached to up to one target creature." (CR 611.2a, 303.7, 608.2b): the control, untap and haste are the first target's;
/// the Role goes onto the second target, a creature of any controller that need not be the first (the two are judged apart: one
/// no longer legal leaves the other's part to happen). It is created under the spell's controller (CR 111.2).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::gain_control_untap_haste(g, item);
    let Some(creature) = targets::object(g, item, 1) else { return Ok(()) };
    tokens::create_role(g, &tokens::WICKED_ROLE, item.controller, creature)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE, up_to_one(targets::creature_filter)], effect),
    ..oracle::TWISTED_FEALTY
};

pub static EXTRA: &[&CardDef] = &[&tokens::WICKED_ROLE];
