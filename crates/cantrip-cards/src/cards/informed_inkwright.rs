use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::REPARTEE};
use cantrip_oracle as oracle;

/// "Create a 1/1 white and black Inkling creature token with flying."
fn inkling(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::INKLING, 1)?;
    Ok(())
}

/// Vigilance is a core keyword. "Repartee — Whenever you cast an instant or sorcery spell that targets a creature, create a
/// 1/1 white and black Inkling creature token with flying."
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        REPARTEE,
        inkling,
        "Repartee — Whenever you cast an instant or sorcery spell that targets a creature, create a 1/1 white and black Inkling creature token with flying.",
    )],
    ..oracle::INFORMED_INKWRIGHT
};

pub static EXTRA: &[&CardDef] = &[&tokens::INKLING];
