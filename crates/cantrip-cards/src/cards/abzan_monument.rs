use cantrip_api::*;
use cantrip_mechanics::lands::sub;
use cantrip_mechanics::select::{basic_land_of_types, up_to, Selection};
use cantrip_mechanics::{endure, greatest_toughness, library, tokens, triggers};
use cantrip_oracle as oracle;

/// "A basic Plains, Swamp, or Forest card", which may be left unfound (CR 701.23b).
const BASIC_PLAINS_SWAMP_OR_FOREST_CARD: Selection =
    Selection::of(up_to(1), &[basic_land_of_types::<{ sub(Subtype::Plains) }, { sub(Subtype::Swamp) }, { sub(Subtype::Forest) }>()])
        .called("basic Plains, Swamp, or Forest");

/// "When this artifact enters, search your library for a basic Plains, Swamp, or Forest card, reveal it, put it into your hand,
/// then shuffle" (CR 701.23).
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_hand(g, item.controller, item.object, BASIC_PLAINS_SWAMP_OR_FOREST_CARD, true)?;
    Ok(())
}

/// "Create an X/X white Spirit creature token, where X is the greatest toughness among creatures you control" (CR 111.3,
/// 608.2h): the creatures as the ability resolves, the sacrificed Monument not being one; a greatest toughness below zero makes
/// X 0 (CR 107.1b), and a creature with no toughness has none to count. The token is X/X as it is made (CR 111.3), never a 0/0
/// for a moment; with no creature X is 0 and the 0/0 token goes to the graveyard (CR 704.5f, ruling 2025-04-04).
fn create_a_spirit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let greatest = greatest_toughness::among_creatures_you_control(g, you) as i32;
    tokens::create_sized(g, item.object, you, &endure::SPIRIT, (greatest, greatest), 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            search,
            "When this artifact enters, search your library for a basic Plains, Swamp, or Forest card, reveal it, put it into your hand, then shuffle.",
        ),
        ability(
            cost("{1}{W}{B}{G}, {T}").and(SACRIFICE_THIS),
            create_a_spirit,
            "{1}{W}{B}{G}, {T}, Sacrifice this artifact: Create an X/X white Spirit creature token, where X is the greatest toughness among creatures you control. Activate only as a sorcery.",
        )
        .sorcery_speed(),
    ],
    ..oracle::ABZAN_MONUMENT
};

pub static EXTRA: &[&CardDef] = &[&endure::SPIRIT];
