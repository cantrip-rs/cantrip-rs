use cantrip_api::*;
use cantrip_mechanics::{earthbend, flashback, permanents_with_subtype_you_control};
use cantrip_oracle as oracle;

/// "Earthbend X, where X is the number of Forests you control" (CR 701.66a, 107.3): X is counted as the spell resolves
/// (CR 608.2h), Forest being a word of the spell's own text (CR 612).
fn earthbend_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let forest = g.words(item.object).subtype(Subtype::Forest);
    let x = permanents_with_subtype_you_control(g, item.controller, forest) as i32;
    earthbend::earthbend(g, item, x)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[earthbend::TARGET], earthbend_x),
    delayed: &[earthbend::RETURN],
    ways: &[flashback::way(cost("{5}{G}"), "Flashback {5}{G}")],
    ..oracle::ROCKALANCHE
};
