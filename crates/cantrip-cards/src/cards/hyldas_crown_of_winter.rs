use cantrip_api::*;
use cantrip_mechanics::{effects, targets, your_turn};
use cantrip_oracle as oracle;

/// "This ability costs {1} less to activate during your turn" (CR 601.2f, 602.2b, 118.7a): generic mana only, judged as the total
/// cost is determined, by whether the activating player is the active player.
fn less_during_your_turn(q: &dyn Query, asker: &Asker, _: Costed, _: &[Target]) -> CostDelta {
    if your_turn(q, asker) { CostDelta::less(1) } else { CostDelta::NONE }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}, {T}"),
            effects::tap_targets,
            "{1}, {T}: Tap target creature. This ability costs {1} less to activate during your turn.",
        )
        .targets(&[targets::CREATURE])
        .cost_change(less_during_your_turn),
        ability(
            cost("{3}").and(SACRIFICE_THIS),
            effects::draw_a_card_for_each_tapped_creature_your_opponents_control,
            "{3}, Sacrifice this artifact: Draw a card for each tapped creature your opponents control.",
        ),
    ],
    ..oracle::HYLDAS_CROWN_OF_WINTER
};
