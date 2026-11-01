use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Untap one or two target creatures. They each get +2/+2 until end of turn." (CR 701.26, 611.2a): those still legal as it
/// resolves (CR 608.2b), untapped and then pumped.
fn untap_and_pump(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::untap_targets(g, item)?;
    effects::targets_get::<2, 2>(g, item)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE.between(1, 2)], untap_and_pump),
    ..oracle::FANCY_FOOTWORK
};
