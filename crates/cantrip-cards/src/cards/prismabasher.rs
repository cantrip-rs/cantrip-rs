use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{targets, vivid};
use cantrip_oracle as oracle;

/// Vivid: X, the number of colors among permanents `asker`'s controller controls (CR 207.2c, 105.1).
fn colors_among_your_permanents(q: &dyn Query, asker: &Asker) -> u16 {
    u16::from(vivid::count(q, asker.controller))
}

/// "Up to X target creatures you control get +X/+X until end of turn, where X is the number of colors among permanents you
/// control" (CR 611.2a, 608.2b): the number of targets is X as the ability goes on the stack (`counted_by`), and the bonus is X as
/// it resolves (the ruling), for each target still legal.
fn targets_get_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = i16::from(vivid::count(g, item.controller));
    let creatures = targets::legal_objects(g, item);
    instruction(g, |g| {
        for &creature in &creatures {
            cantrip_mechanics::pump(g, item.object, creature, x, x);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        targets_get_x,
        "Vivid — When this creature enters, up to X target creatures you control get +X/+X until end of turn, where X is the number of colors among permanents you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL.between(0, TargetSpec::COUNTED)])
    .counted_by(colors_among_your_permanents)],
    ..oracle::PRISMABASHER
};
