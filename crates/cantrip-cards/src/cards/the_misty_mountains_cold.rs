use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// The 6/6 red Dragon creature token with flying. The words of the Saga's text make its color and creature type
/// (CR 612.4, 111.4), so nothing here reads them.
static DRAGON: CardDef = CardDef {
    name: "Dragon Token",
    token_name: Some(&[Subtype::Dragon]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Dragon]),
    power: Some(6),
    toughness: Some(6),
    keywords: Keywords::of(&[Keyword::Flying]),
    text: "Flying",
    ..CardDef::BLANK
};

/// "Create a Treasure token. Then if you control four or more Treasures, sacrifice this Saga. If you do, create a 6/6 red
/// Dragon creature token with flying." (CR 111.10a, 701.21a, 714.4) The Treasures are counted after the new one has
/// entered, whoever made them and token or not; the Dragon comes only if the Saga was sacrificed (CR 400.7: a Saga that
/// has already left the battlefield cannot be).
fn treasure_and_dragon(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_treasure(g, item.controller)?;
    let treasures = tokens::treasures_controlled(g, item.controller).len();
    if treasures >= 4 && !g.sacrifice(&[item.object]).is_empty() {
        tokens::create(g, item.controller, &DRAGON, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter_ability(1),
        chapter_ability(2),
        chapter_ability(3),
        chapter_ability(4),
    ],
    ..oracle::THE_MISTY_MOUNTAINS_COLD
};

const fn chapter_ability(n: u8) -> AbilityDef {
    chapter(n, treasure_and_dragon,"I, II, III, IV \u{2014} Create a Treasure token. Then if you control four or more Treasures, sacrifice this Saga. If you do, create a 6/6 red Dragon creature token with flying.")
}

pub static EXTRA: &[&CardDef] = &[&DRAGON];
