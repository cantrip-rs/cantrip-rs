use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::destroy_first_target, "When this artifact enters, destroy target creature.").targets(&[targets::CREATURE]),
        ability(
            cost("{2}, {T}").and(SACRIFICE_THIS),
            |g, item| effects::gain_life(g, item, 3),
            "{2}, {T}, Sacrifice this artifact: You gain 3 life.",
        ),
    ],
    ..oracle::ANCHOVY_BANANA_PIZZA
};
