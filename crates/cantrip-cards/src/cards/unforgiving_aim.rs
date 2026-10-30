use cantrip_api::*;
use cantrip_mechanics::{destroy_target, targets, tokens};
use cantrip_oracle as oracle;

/// "Destroy target creature with flying."
fn destroy_flier(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

/// "Destroy target enchantment."
fn destroy_enchantment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

/// "Create a 2/2 black and green Elf creature token" (CR 111.1).
fn create_elf(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::ELF_BLACK_GREEN, 1)?;
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(destroy_flier, "Destroy target creature with flying.").targets(&[targets::CREATURE_WITH_FLYING]),
    mode(destroy_enchantment, "Destroy target enchantment.").targets(&[targets::ENCHANTMENT]),
    mode(create_elf, "Create a 2/2 black and green Elf creature token."),
]);

/// The Elf token the third mode makes.
pub static EXTRA: &[&CardDef] = &[&tokens::ELF_BLACK_GREEN];

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::UNFORGIVING_AIM };
