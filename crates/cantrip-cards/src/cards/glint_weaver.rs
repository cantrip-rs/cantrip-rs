use cantrip_api::*;
use cantrip_mechanics::{effects, greatest_toughness, targets, triggers};
use cantrip_oracle as oracle;

/// "Distribute three +1/+1 counters among one, two, or three target creatures, then you gain life equal to the greatest toughness among
/// creatures you control" (CR 601.2d, 122.1, 608.2b, 107.1b): the counters first, each target its share as the ability was put on the stack
/// (a target illegal as it resolves is left out with its share), then the life, counted afterward, so the counters count.
fn distribute_then_gain_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::distribute_plus_one_counters(g, item)?;
    let life = greatest_toughness::among_creatures_you_control(g, item.controller) as i32;
    effects::gain_life(g, item, life)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        distribute_then_gain_life,
        "When this creature enters, distribute three +1/+1 counters among one, two, or three target creatures, then you gain life equal to the greatest toughness among creatures you control.",
    )
    .targets(&[targets::CREATURE.between(1, 3)])
    .divide(Divide { total: 3, reads: None })],
    ..oracle::GLINT_WEAVER
};
