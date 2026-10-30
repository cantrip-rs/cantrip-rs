use cantrip_api::*;
use cantrip_mechanics::statics::plus_pt;
use cantrip_oracle as oracle;

/// "As long as there are two or more creature cards in your graveyard" (CR 604.1, 611.3a), affecting only this creature: the cards
/// in its controller's graveyard that are creature cards, as the layers make them, asked again whenever characteristics are computed.
fn itself_with_two_creature_cards_in_your_graveyard(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    let state = view.state();
    source == affected && state.zone(Zone::graveyard(state.obj(source).controller)).iter().filter(|&&card| view.chars(card).is(CardType::Creature)).count() >= 2
}

/// "As long as there are two or more creature cards in your graveyard, this creature gets +2/+2 and is all creature types."
/// "{T}: Add one mana of any color." (CR 106.1b, 605.1a)
pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color.")],
    statics: &[
        static_ability(Layer::PtModify, itself_with_two_creature_cards_in_your_graveyard, plus_pt::<2, 2>, "As long as there are two or more creature cards in your graveyard, this creature gets +2/+2."),
        static_ability(Layer::Type, itself_with_two_creature_cards_in_your_graveyard, cantrip_mechanics::statics::all_creature_types, "As long as there are two or more creature cards in your graveyard, this creature is all creature types."),
    ],
    ..oracle::UNDERCOVER_SKRULL
};
