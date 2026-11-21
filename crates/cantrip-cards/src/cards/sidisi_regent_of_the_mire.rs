use cantrip_api::*;
use cantrip_mechanics::{effects, targets::{self, filters}};
use cantrip_oracle as oracle;

/// "Target creature card with mana value X plus 1 from your graveyard" (CR 107.3a, 202.3, 115.1): X is the mana value of the creature
/// sacrificed, announced first (CR 601.2b) and then read by the target chosen after it (`targets_after_x`, CR 601.2c), each
/// activation's own (CR 107.3k) and judged again as the ability resolves (CR 608.2b). Before X is announced, when it is only asked
/// whether some announcement could have a legal target, any creature card in the graveyard is allowed, since some X would allow
/// it. A card with {X} in its mana cost in a graveyard has X of 0 (the ruling), which `Query::mana_value` gives.
fn creature_card_in_your_graveyard_with_mana_value_x_plus_1(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Creature])
        && matches!(t, Target::Object(id) if q.announced_x().is_none_or(|x| q.mana_value(id) == x + 1))
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{T}").and(measured(CostTake::Sacrifice, CostMeasure::ManaValue, filters::another_creature)),
        effects::return_first_target_to_battlefield,
        "{T}, Sacrifice a creature you control with mana value X other than Sidisi: Return target creature card with mana value X plus 1 from your graveyard to the battlefield. Activate only as a sorcery.",
    )
    .sorcery_speed()
    .targets(&[target(creature_card_in_your_graveyard_with_mana_value_x_plus_1)])
    .targets_after_x()],
    ..oracle::SIDISI_REGENT_OF_THE_MIRE
};
