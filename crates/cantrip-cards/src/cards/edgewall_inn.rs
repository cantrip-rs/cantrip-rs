use cantrip_api::*;
use cantrip_mechanics::{
    effects,
    lands::{self, enters_tapped_choosing_a_color},
    library, targets,
};
use cantrip_oracle as oracle;

/// "Target card that has an Adventure from your graveyard" (CR 115.1, 715.1): a card in your graveyard that has an Adventure,
/// whether or not it was ever cast as one (`library::card_has_an_adventure`).
fn card_that_has_an_adventure_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[]) && matches!(t, Target::Object(id) if library::card_has_an_adventure(q, id))
}

pub static CARD: CardDef = CardDef {
    // "This land enters tapped. As this land enters, choose a color" (CR 614.1c) and "{T}: Add one mana of the chosen color"
    // (CR 605.1a, 607.2d): the shared land template.
    enters: Some(enters_tapped_choosing_a_color),
    abilities: &[
        lands::TAP_FOR_THE_CHOSEN_COLOR,
        ability(
            cost("{3}").and(TAP).and(SACRIFICE_THIS),
            effects::return_first_target_to_hand,
            "{3}, {T}, Sacrifice this land: Return target card that has an Adventure from your graveyard to your hand.",
        )
        .targets(&[target(card_that_has_an_adventure_in_your_graveyard)]),
    ],
    ..oracle::EDGEWALL_INN
};
