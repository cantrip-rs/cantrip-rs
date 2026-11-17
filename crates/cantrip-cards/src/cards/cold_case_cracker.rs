use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

/// "Investigate." (CR 701.16a): its controller creates a Clue token.
fn investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)
}

/// Flying is a core keyword read from its characteristics. "When this creature dies, investigate." (CR 700.4, 701.16a).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_DIES, investigate, "When this creature dies, investigate.")],
    ..oracle::COLD_CASE_CRACKER
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
