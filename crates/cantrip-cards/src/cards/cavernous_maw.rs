use cantrip_api::*;
use cantrip_mechanics::{animate, subtype_cards_in_graveyard, Animation};
use cantrip_oracle as oracle;

/// "{2}: This land becomes a 3/3 Elemental creature until end of turn. It's still a Cave land." (CR 613.1d, 613.4b, 611.2a): it
/// sets no color, so the land keeps the colors it has (none), and the Elemental type is laid over its types as Mishra's Factory's
/// Assembly-Worker is, the Elemental word being read through the words of this card's text (CR 612).
fn becomes_3_3_elemental(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let animation = Animation {
        power: 3,
        toughness: 3,
        colors: None,
        types: TypeSet::NONE,
        subtypes: &[g.words(item.object).subtype(Subtype::Elemental)],
        all_creature_types: false,
        replaces_creature_types: false,
        keywords: Keywords::NONE,
        ability: None,
    };
    animate(g, item.object, item.object, &animation, Duration::EndOfTurn);
    Ok(())
}

/// "Activate only if the number of other Caves you control plus the number of Cave cards in your graveyard is three or greater"
/// (CR 602.5b): counted as the ability is activated, "Cave" read through the words of this card's text (CR 612); the land itself is
/// not one of the "other" Caves, and a card of an opponent's in their graveyard is not counted.
fn three_or_more_caves(q: &dyn Query, asker: &Asker) -> bool {
    let cave = q.words(asker.source).subtype(Subtype::Cave);
    let others = q.permanents_of(asker.controller).into_iter().filter(|&id| id != asker.source && q.chars(id).has_subtype(cave)).count();
    others + subtype_cards_in_graveyard(q, asker.controller, cave) >= 3
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(
            cost("{2}"),
            becomes_3_3_elemental,
            "{2}: This land becomes a 3/3 Elemental creature until end of turn. It's still a Cave land. Activate only if the number of other Caves you control plus the number of Cave cards in your graveyard is three or greater.",
        )
        .condition(three_or_more_caves),
    ],
    ..oracle::CAVERNOUS_MAW
};
