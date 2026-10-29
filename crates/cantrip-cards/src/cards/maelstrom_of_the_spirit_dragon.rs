use cantrip_api::*;
use cantrip_mechanics::library::{Find, search_to_hand};
use cantrip_oracle as oracle;

/// "Search your library for a Dragon card, reveal it, put it into your hand, then shuffle." (CR 701.23): a card with the
/// subtype Dragon, which the search may fail to find (CR 701.23b). "Dragon" is read through the land's words as it last
/// was, since it was sacrificed to pay for this (CR 612, 113.7a).
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let dragon = g.words(item.object).subtype(Subtype::Dragon);
    search_to_hand(g, item.controller, item.object, Find::ONE.revealed(), |q, c| q.chars(c).has_subtype(dragon))?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        // "Spend this mana only to cast a Dragon spell or an Omen spell" (CR 106.6, 205.3): a spell that has either
        // subtype as it is cast; the words are the land's own, as the mana is added (CR 612).
        mana_ability(
            TAP,
            add_any_color(1).only_for(ManaRestriction::Subtypes(SubtypeSpells::of_any(&[Subtype::Dragon, Subtype::Omen]))),
            "{T}: Add one mana of any color. Spend this mana only to cast a Dragon spell or an Omen spell.",
        ),
        ability(
            cost("{4}, {T}").and(SACRIFICE_THIS),
            search,
            "{4}, {T}, Sacrifice this land: Search your library for a Dragon card, reveal it, put it into your hand, then shuffle.",
        ),
    ],
    ..oracle::MAELSTROM_OF_THE_SPIRIT_DRAGON
};
