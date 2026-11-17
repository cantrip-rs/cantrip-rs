use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Creature spells you cast cost {1} less to cast." (CR 601.2f).
fn creature_spells_cost_1_less(
    q: &dyn Query,
    source: ObjectId,
    player: PlayerId,
    costed: Costed,
    _: &[Target],
) -> CostDelta {
    cantrip_mechanics::cost_reduction::spells_you_cast(q, source, player, costed, CardType::Creature, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::return_first_target_to_hand, "When Honest Rutstein enters, return target creature card from your graveyard to your hand.")
            .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD]),
    ],
    cost_changes: &[creature_spells_cost_1_less],
    ..oracle::HONEST_RUTSTEIN
};
