use cantrip_api::*;
use cantrip_mechanics::triggers::{ward, THIS_ENTERS};
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "You gain X life, where X is the greatest power among Giants you control." X is worked out once, as the ability resolves
/// (the ruling of 2025-11-17), from the Giants on the battlefield then, through the layers; with none (this creature gone, or
/// no longer a Giant) or only negative powers, X is 0 (CR 107.1b).
fn gain_greatest_giant_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let giant = g.words(item.object).subtype(Subtype::Giant);
    let greatest = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| g.chars(id).has_subtype(giant))
        .filter_map(|id| g.chars(id).power)
        .fold(0, i16::max);
    effects::gain_life(g, item, i32::from(greatest))
}

/// Reach and vigilance are core keywords.
pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<2>("Ward {2}"),
        triggered(
            THIS_ENTERS,
            gain_greatest_giant_power,
            "When this creature enters, you gain X life, where X is the greatest power among Giants you control.",
        ),
    ],
    ..oracle::PUMMELER_FOR_HIRE
};
