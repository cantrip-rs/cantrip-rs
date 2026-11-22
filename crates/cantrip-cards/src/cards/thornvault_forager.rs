use cantrip_api::*;
use cantrip_mechanics::{forage, library, mana, select};
use cantrip_oracle as oracle;

/// "A Squirrel card" (CR 205.3, 701.23b): the library may be searched and no card found.
const SQUIRREL_CARD: select::Selection = select::Selection::of(select::up_to(1), &[select::subtype_card(Subtype::Squirrel)]).called("Squirrel");

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::G), "{T}: Add {G}."),
        // A mana ability whose cost is a choice (CR 605.1a, 601.2b): forage is exiling three cards from the graveyard or
        // sacrificing a Food, and mana is added as it is activated, with no stack to respond on (CR 605.3b).
        mana_ability(TAP, add_by(|g, asker, _| mana::add_in_any_combination(g, asker, ColorSet(0b11111), 2)).yields_any(2), "{T}, Forage: Add two mana in any combination of colors.")
            .pay_one_of(&[forage::EXILE_THREE_CARDS, forage::SACRIFICE_A_FOOD]),
        ability(
            cost("{3}{G}, {T}"),
            |g, item| library::find_to_hand(g, item.controller, item.object, SQUIRREL_CARD, true).map(drop),
            "{3}{G}, {T}: Search your library for a Squirrel card, reveal it, put it into your hand, then shuffle.",
        ),
    ],
    ..oracle::THORNVAULT_FORAGER
};
