use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens::HUMAN};
use cantrip_oracle as oracle;

/// "Destroy target nonland permanent. Its controller creates a 1/1 white Human creature token."
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::destroy_and_its_controller_creates(g, item, &HUMAN)
}

pub static EXTRA: &[&CardDef] = &[&HUMAN];

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::NONLAND_PERMANENT], effect),
    ..oracle::STROKE_OF_MIDNIGHT
};
