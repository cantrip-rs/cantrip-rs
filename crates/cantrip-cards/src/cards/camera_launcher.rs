use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

pub use cantrip_mechanics::tokens::THOPTER;

/// "Put a +1/+1 counter on this creature. Create a 1/1 colorless Thopter artifact creature token with flying." (CR 122.1,
/// 111.1): in order, the counter going on the source and the Thopter made whether or not it is still there (CR 608.2c).
fn counter_and_thopter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counter_on_this(g, item)?;
    g.end_sentence();
    tokens::create_a_thopter(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}"),
        counter_and_thopter,
        "Exhaust \u{2014} {3}: Put a +1/+1 counter on this creature. Create a 1/1 colorless Thopter artifact creature token with flying. (Activate each exhaust ability only once.)",
    )
    .exhaust()],
    ..oracle::CAMERA_LAUNCHER
};

pub static EXTRA: &[&CardDef] = &[&THOPTER];
