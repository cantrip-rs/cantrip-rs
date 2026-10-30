use cantrip_api::*;
use cantrip_mechanics::statics::{all_creature_types, itself_with_two_creature_cards_in_your_graveyard, plus_pt};
use cantrip_oracle as oracle;

/// "As long as there are two or more creature cards in your graveyard, this creature gets +2/+2 and is all creature types."
/// "{T}: Add one mana of any color." (CR 106.1b, 605.1a)
pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color.")],
    statics: &[
        static_ability(Layer::PtModify, itself_with_two_creature_cards_in_your_graveyard, plus_pt::<2, 2>, "As long as there are two or more creature cards in your graveyard, this creature gets +2/+2."),
        static_ability(Layer::Type, itself_with_two_creature_cards_in_your_graveyard, all_creature_types, "As long as there are two or more creature cards in your graveyard, this creature is all creature types."),
    ],
    ..oracle::UNDERCOVER_SKRULL
};
