use cantrip_api::*;
use cantrip_mechanics::{effects, flurry, targets};
use cantrip_oracle as oracle;

/// "This creature deals 2 damage to target opponent. Scry 1." (CR 120.3, 701.22, 608.2b): two sentences. A target that is no longer
/// legal makes the whole ability not resolve (it is the only target), so the scry is lost with it.
fn damage_and_scry(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_first_target::<2>(g, item)?;
    g.end_sentence();
    effects::scry(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[flurry::trigger(
        "Whenever you cast your second spell each turn, this creature deals 2 damage to target opponent. Scry 1.",
        damage_and_scry,
    )
    .targets(&[targets::OPPONENT])],
    ..oracle::IRON_FIST_PULVERIZER
};
