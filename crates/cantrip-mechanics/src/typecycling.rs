//! Typecycling [cost] (CR 702.29e): "[Type]cycling [cost]" means "[Cost], Discard this card: Search your library for a
//! [type] card, reveal it, put it into your hand. Then shuffle." Eagles of the North (Plainscycling), Generous Ent
//! (Forestcycling), L\u{f3}rien Revealed (Islandcycling), Oliphaunt (Mountaincycling), Troll of Khazad-d\u{fb}m
//! (Swampcycling) and Hobbit Hole (Halflingcycling) print it. Plain cycling (CR 702.29a) is `triggers::cycling`; basic landcycling, whose type is the
//! basic supertype, is `basic_landcycling`.
//!
//! A card with typecycling lists `ability(subtype, cost, text)` among its `abilities`; `subtype` is the basic land type
//! (or, for Halflingcycling, the creature type) it searches for.

use crate::search_library;
use cantrip_api::*;

macro_rules! search_for {
    ($name:ident, $subtype:expr) => {
        fn $name(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
            let player = item.controller;
            let found = search_library(g, player, player, item.object, SearchDestination::Hand, 1, true, |q, c| q.chars(c).has_subtype($subtype))?;
            g.reveal(player, &found);
            let moves: Vec<(ObjectId, Zone)> = found.iter().map(|&c| (c, Zone::hand(player))).collect();
            g.move_together_revealed(&moves, MoveReason::Search);
            g.shuffle(player);
            Ok(())
        }
    };
}

search_for!(search_plains, Subtype::Plains);
search_for!(search_island, Subtype::Island);
search_for!(search_swamp, Subtype::Swamp);
search_for!(search_mountain, Subtype::Mountain);
search_for!(search_forest, Subtype::Forest);
search_for!(search_halfling, Subtype::Halfling);

/// The typecycling ability for a basic land type or Halfling, from the hand. `text` is the ability's full text, which a scenario's
/// `Ability:` matches.
pub const fn ability(subtype: Subtype, mana: ManaCost, text: &'static str) -> AbilityDef {
    let effect = match subtype {
        Subtype::Plains => search_plains,
        Subtype::Island => search_island,
        Subtype::Swamp => search_swamp,
        Subtype::Mountain => search_mountain,
        Subtype::Forest => search_forest,
        Subtype::Halfling => search_halfling,
        _ => panic!("typecycling is implemented for the basic land types and Halfling"),
    };
    cantrip_api::ability(Cost::mana(mana).and(DISCARD_THIS), effect, text)
        .from(ZoneKind::Hand)
        .cycling()
}

/// Basic landcycling (CR 702.29e): "Search your library for a basic land card, reveal it, put it into your hand, then
/// shuffle." The basic supertype, not a basic land type, is what it looks for (CR 205.4c): Wastes qualifies and a
/// nonbasic Forest does not.
fn search_basic_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    crate::library::search_to_hand(g, item.controller, item.object,
        crate::library::Find::ONE.revealed(), crate::lands::is_basic_land_card)?;
    Ok(())
}

/// The basic landcycling ability, from the hand (CR 702.29e). Apex Witchstalker, Awaken the Inferno, Hexhaven Battalion,
/// Undulating Witness and Vinelasher Adept print it.
pub const fn basic_landcycling(mana: ManaCost, text: &'static str) -> AbilityDef {
    cantrip_api::ability(Cost::mana(mana).and(DISCARD_THIS), search_basic_land, text).from(ZoneKind::Hand).cycling()
}
