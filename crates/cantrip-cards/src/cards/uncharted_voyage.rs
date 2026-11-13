use cantrip_api::*;
use cantrip_mechanics::{effects, library, targets};
use cantrip_oracle as oracle;

/// "Target creature's owner puts it on their choice of the top or bottom of their library. Surveil 1." The surveil is its own
/// sentence (CR 608.2c), after the creature is in the library: the top card it looks at may be the creature itself, put on top.
fn put_on_top_or_bottom_then_surveil(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::owner_puts_first_target_on_top_or_bottom(g, item)?;
    g.end_sentence();
    effects::surveil(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], put_on_top_or_bottom_then_surveil),
    ..oracle::UNCHARTED_VOYAGE
};
