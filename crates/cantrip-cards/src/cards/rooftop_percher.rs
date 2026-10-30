use cantrip_api::*;
use cantrip_mechanics::{targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Exile up to two target cards from graveyards. You gain 3 life." (CR 115.1, 115.3, 608.2b): one word "target", so two
/// different cards (one slot of up to two); the targets still in their graveyards are exiled together, from any graveyards; a
/// target that became illegal is left alone and the life is still gained as long as one target remains legal (or none was
/// chosen).
fn exile_and_gain_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.exile_all(&targets::legal_objects(g, item));
    g.gain_life(item.controller, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, exile_and_gain_life, "When this creature enters, exile up to two target cards from graveyards. You gain 3 life.")
        .targets(&[targets::CARD_IN_A_GRAVEYARD.between(0, 2)])],
    ..oracle::ROOFTOP_PERCHER
};
