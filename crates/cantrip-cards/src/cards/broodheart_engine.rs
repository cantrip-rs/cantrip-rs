use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_UPKEEP;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature or Vehicle card from your graveyard" (CR 115.1, 205.3g): a card in the asker's own graveyard that is a creature
/// card or has the Vehicle subtype.
fn creature_or_vehicle_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::creature_card_or_subtype_card_in_your_graveyard(q, asker, t, Subtype::Vehicle)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(YOUR_UPKEEP, |g, item| effects::surveil(g, item, 1), "At the beginning of your upkeep, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)"),
        ability(
            cost("{2}{B}{G}, {T}").and(SACRIFICE_THIS),
            effects::return_first_target_to_battlefield,
            "{2}{B}{G}, {T}, Sacrifice this artifact: Return target creature or Vehicle card from your graveyard to the battlefield. Activate only as a sorcery.",
        )
        .sorcery_speed()
        .targets(&[target(creature_or_vehicle_card_in_your_graveyard)]),
    ],
    ..oracle::BROODHEART_ENGINE
};
