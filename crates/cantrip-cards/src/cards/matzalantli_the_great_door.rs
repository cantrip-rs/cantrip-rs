use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Activate only if there are four or more permanent types among cards in your graveyard" (CR 602.5b, 110.4a, 205.2a): checked
/// as the ability is activated. Artifact, battle, creature, enchantment, land and planeswalker are the permanent types, each
/// counted once however many cards have it, a card with several types giving each of them; an instant, sorcery or kindred
/// type is not one.
fn four_permanent_types_in_your_graveyard(q: &dyn Query, asker: &Asker) -> bool {
    let graveyard = q.state().zone(Zone::graveyard(asker.controller));
    targets::PERMANENT_CARD_TYPES.iter().filter(|&&t| graveyard.iter().any(|&card| q.chars(card).is(t))).count() >= 4
}

/// "{T}: Add X mana of any one color, where X is the number of permanent cards in your graveyard" (CR 605.1a, 110.4a):
/// counted as the ability resolves; with none, no mana is added.
fn add_mana_for_permanent_cards_in_your_graveyard(g: &mut dyn Game, asker: &Asker, color: Option<Color>) -> Result<(), Suspend> {
    let x = targets::permanent_cards_in_graveyard(&*g, asker.controller) as u32;
    if let Some(color) = color
        && x > 0
    {
        g.add_mana(asker.controller, color.mana(), x);
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add_by(add_mana_for_permanent_cards_in_your_graveyard).choose_color(),
        "Fathomless descent \u{2014} {T}: Add X mana of any one color, where X is the number of permanent cards in your graveyard.",
    )],
    ..oracle::THE_CORE
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, effects::loot, "{T}: Draw a card, then discard a card."),
        ability(
            cost("{4}").and(TAP),
            effects::transform_self,
            "{4}, {T}: Transform Matzalantli. Activate only if there are four or more permanent types among cards in your graveyard.",
        )
        .condition(four_permanent_types_in_your_graveyard),
    ],
    back: Some(&BACK),
    ..oracle::MATZALANTLI_THE_GREAT_DOOR
};
