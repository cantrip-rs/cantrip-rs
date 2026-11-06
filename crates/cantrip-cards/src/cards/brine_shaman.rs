use cantrip_api::*;
use cantrip_mechanics::{counter_target, effects, targets};
use cantrip_oracle as oracle;

/// Counter the targeted creature spell (CR 701.6a).
fn counter_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counter_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            TAP.and(sacrifice(1, targets::filters::creature)),
            effects::target_creature_gets::<2, 2>,
            "{T}, Sacrifice a creature: Target creature gets +2/+2 until end of turn.",
        )
        .targets(&[targets::CREATURE]),
        ability(
            cost("{1}{U}{U}").and(sacrifice(1, targets::filters::creature)),
            counter_creature,
            "{1}{U}{U}, Sacrifice a creature: Counter target creature spell.",
        )
        .targets(&[targets::CREATURE_SPELL]),
    ],
    ..oracle::BRINE_SHAMAN
};
