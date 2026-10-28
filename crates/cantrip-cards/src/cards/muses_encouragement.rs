use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Create a 3/3 blue and red Elemental creature token with flying. Surveil 2."
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::ELEMENTAL_3_3_BLUE_RED, 1)?;
    g.end_sentence();
    effects::surveil(g, item, 2)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::MUSES_ENCOURAGEMENT };

pub static EXTRA: &[&CardDef] = &[&tokens::ELEMENTAL_3_3_BLUE_RED];
