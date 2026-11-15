use cantrip_api::*;
use cantrip_mechanics::{creature_cards_in_graveyard, fight, targets};
use cantrip_oracle as oracle;

/// "This spell costs {2} less to cast if there are two or more creature cards in your graveyard." (CR 601.2f, 118.7a): counted as
/// the total cost is determined; generic mana only.
fn less_with_two_creature_cards(q: &dyn Query, _: ObjectId, controller: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    if creature_cards_in_graveyard(q, controller) >= 2 { CostDelta::less(2) } else { CostDelta::NONE }
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[
            targets::CREATURE_YOU_CONTROL.labelled("Creature you control to deal damage equal to twice its power"),
            targets::CREATURE_AN_OPPONENT_CONTROLS.labelled("Creature an opponent controls to be dealt that damage"),
        ],
        fight::first_target_bites_second_for_twice_its_power,
    ),
    own_cost_changes: &[less_with_two_creature_cards],
    ..oracle::PUNISHING_PUNCH
};
