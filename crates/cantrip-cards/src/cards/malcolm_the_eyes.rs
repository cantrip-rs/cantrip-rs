use cantrip_api::*;
use cantrip_mechanics::{flurry, tokens};
use cantrip_oracle as oracle;

/// "Investigate." (CR 701.16a): its controller creates a Clue.
fn investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[flurry::trigger("Whenever you cast your second spell each turn, investigate.", investigate)],
    ..oracle::MALCOLM_THE_EYES
};
