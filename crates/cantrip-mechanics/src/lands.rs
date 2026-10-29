//! Land templates: the lands of a printed cycle differ in their colors and little else (CR 305). A land's file names its
//! colors and uses these. The colors are numbered as `Color` and `Mana` number them (`W`..`G` below), the basic land
//! types as `Subtype`'s discriminant (`sub(Subtype::Plains)`), because a function pointer cannot capture them.
//!
//! The "enters tapped" part of a land is `mana::enters_tapped`, `mana::enters_tapped_unless_pay_two_life` (shock lands) or
//! `enters_tapped_unless_you_control`, `enters_tapped_unless_you_control_either` or
//! `enters_tapped_unless_opponents_control` below; the card sets it in `enters`.

use crate::library::{ask_may_search, find_to_battlefield, find_to_hand, find_to_top, search_to_battlefield, Find};
use crate::select;
use crate::{choose, effects, mana, triggers};
use cantrip_api::*;

/// "A basic land card" (CR 205.4a, 305.6): a land card with the basic supertype, as the library search of every fetch below
/// asks it of the cards in the library.
pub fn is_basic_land_card(q: &dyn Query, card: ObjectId) -> bool {
    let chars = q.chars(card);
    chars.is(CardType::Land) && chars.supertypes.basic
}

/// A subtype as the number the generic land templates take: `sub(Subtype::Island)`.
pub const fn sub(subtype: Subtype) -> u16 {
    subtype as u16
}

/// The subtype a template's number names, as `source`'s text has that word now (CR 612.2).
pub fn word(q: &dyn Query, source: ObjectId, n: u16) -> Subtype {
    q.words(source).subtype(printed(n))
}

/// The printed subtype a template's number names (`sub`), before any text change reads it (CR 612.2).
pub fn printed(n: u16) -> Subtype {
    Subtype::ALL.into_iter().find(|&s| s as u16 == n).expect("a subtype's number")
}

/// Whether `controller` controls a permanent with the land type `land_type`, through the layers (CR 305.6, 613). Arena of
/// Glory, Castle Locthwain and Shifting Woodland's "unless you control a [land type]" and Cori Mountain Monastery's "unless
/// you control a Plains or an Island" ask it.
pub fn controls_land_type(q: &dyn Query, controller: PlayerId, land_type: Subtype) -> bool {
    q.state().battlefield().iter().any(|&id| q.controller(id) == controller && q.chars(id).has_subtype(land_type))
}

/// "This land enters tapped. As it enters, choose a color." (CR 614.1c-d, 614.12a): both as it enters, the color kept on the
/// permanent (CR 607.2d). Sunken Citadel and Valgavoth's Lair print it.
pub fn enters_tapped_choosing_a_color(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.tapped = true;
    entering.chosen_color = Some(crate::choose_color_for(g, entering.controller, entering.object, ColorUse::Benefit)?);
    Ok(())
}

/// "Mana of the chosen color" (CR 106.5): the one color chosen as it entered by the text this ability is of (CR 607.2d,
/// 607.5), none if none was (a land that became a copy of it without entering), and then the ability adds no mana. "The chosen color" is no color word, so a text-changing effect leaves it
/// as it is (CR 612.2). The colors for a mana ability's `colors_from`: Sunken Citadel, Valgavoth's Lair, Heraldic Banner and Room of Refuge.
pub fn chosen_color(q: &dyn Query, asker: &Asker) -> ColorSet {
    q.state().chosen_color(q.linked_to(asker)).map_or(ColorSet::NONE, ColorSet::of)
}

/// "This land enters tapped unless you control a [land type]" (CR 614.12, 614.1d): read on the battlefield as it is while the land
/// enters, so the land itself is not counted, through the layers (a land of any kind may have the type). Arena of Glory
/// (Mountain), Castle Locthwain (Swamp) and Shifting Woodland (Forest) print it.
pub fn enters_tapped_unless_you_control<const TYPE: u16>(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let land_type = word(g, entering.object, TYPE);
    if !controls_land_type(g, entering.controller, land_type) {
        entering.tapped = true;
    }
    Ok(())
}

/// "This land enters tapped unless you control a planeswalker" (CR 614.12, 614.1d): read on the battlefield as it is while the land
/// enters, through the layers; a planeswalker entering simultaneously is not counted, and only the controller's own
/// planeswalkers count. The ten lands in the Annex and Commons cycle print it.
pub fn enters_tapped_unless_you_control_a_planeswalker(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let controls_planeswalker = g
        .permanents_of(entering.controller)
        .into_iter()
        .any(|id| g.obj(id).became.is_none() && g.chars(id).is(CardType::Planeswalker));
    if !controls_planeswalker {
        entering.tapped = true;
    }
    Ok(())
}

/// "This land enters tapped unless you control a basic land" (CR 614.1d, 205.4a): the supertype, not a land type, read
/// on the battlefield as it is while the land enters, so the land itself is not counted. Abandoned Air Temple and Agna
/// Qel'a print it.
pub fn enters_tapped_unless_you_control_basic_land(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let controls_basic = g
        .permanents_of(entering.controller)
        .into_iter()
        .any(|id| g.obj(id).became.is_none() && g.chars(id).is(CardType::Land) && g.chars(id).supertypes.basic);
    if !controls_basic {
        entering.tapped = true;
    }
    Ok(())
}

/// "This land enters tapped unless you control two or more basic lands" (CR 614.12, 614.1d, 205.4c): the supertype, not a land
/// type, read on the battlefield as it is while the land enters, so the land itself - and any land entering at the same time,
/// which the core leaves out as not yet on the battlefield - is not counted; `became.is_none()` drops the objects that already
/// moved. The five battle lands print it (Prairie Stream, Sunken Hollow, Smoldering Marsh, Cinder Glade, Canopy Vista), and so
/// does the enemy-color cycle that completes them (Radiant Summit, Scorched Geyser, Sodden Verdure, Eclipsed Steppe, Vernal Fen).
pub fn enters_tapped_unless_two_or_more_basic_lands(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let basics = g
        .permanents_of(entering.controller)
        .into_iter()
        .filter(|&id| g.obj(id).became.is_none() && g.chars(id).is(CardType::Land) && g.chars(id).supertypes.basic)
        .count();
    if basics < 2 {
        entering.tapped = true;
    }
    Ok(())
}

/// "This land enters tapped unless your opponents control [N] or more lands" (CR 614.1d): judged as the land enters,
/// against the game as it is before any permanent entering at the same time moves (CR 614.12, 101.4), so a land an
/// opponent controls that enters alongside it is not counted, and across every opponent's lands, never its controller's
/// own. The eight Turbulent lands print it with N = 8 (Turbulent Crater, Shore, Wetlands, Fen, Moor, Springs, Steppe
/// and Wilderness).
pub fn enters_tapped_unless_opponents_control<const N: usize>(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let opponents: Vec<PlayerId> = g.state().opponents(entering.controller).collect();
    let lands = opponents
        .into_iter()
        .flat_map(|player| g.permanents_of(player))
        .filter(|&id| g.obj(id).became.is_none() && g.chars(id).is(CardType::Land))
        .count();
    if lands < N {
        entering.tapped = true;
    }
    Ok(())
}

/// "This land enters tapped unless you control a [land type] or a [land type]" (CR 614.1d, 614.12): as `enters_tapped_unless_you_control`
/// with two land types, each read through the layers and the land's words (CR 612.2). Cori Mountain Monastery (Plains or
/// Island) and Mistrise Village (Mountain or Forest) print it.
pub fn enters_tapped_unless_you_control_either<const A: u16, const B: u16>(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let (a, b) = (word(g, entering.object, A), word(g, entering.object, B));
    if !controls_land_type(g, entering.controller, a) && !controls_land_type(g, entering.controller, b) {
        entering.tapped = true;
    }
    Ok(())
}

/// "Destroy target [permanent]. Its controller may search their library for a basic land card, put it onto the battlefield
/// [tapped], then shuffle": the permanent `item` targeted in `slot`, if it is still a legal target (CR 608.2b). Its controller
/// is the one it last had (CR 608.2h), asked to search after the destruction and even when the permanent was not destroyed
/// (indestructible, regenerated); declining skips the search and the shuffle. Erode (tapped), Ghost Quarter (untapped) and
/// Avengers Disassembled (tapped) print it.
pub fn destroy_target_its_controller_may_fetch_basic(g: &mut dyn Game, item: &StackItem, slot: usize, tapped: bool) -> Result<(), Suspend> {
    let Some(permanent) = crate::targets::object(g, item, slot) else { return Ok(()) };
    let owner = g.controller(permanent);
    g.destroy(permanent);
    may_fetch_basic(g, owner, item.object, tapped)
}

/// "[Player] may search their library for a basic land card, put it onto the battlefield [tapped], then shuffle": asked
/// first whether to, and declining skips the search and the shuffle (CR 701.23). Ghost Quarter, Erode and Avengers
/// Disassembled (through `destroy_target_its_controller_may_fetch_basic`) and Demolition Field's own search print it.
pub fn may_fetch_basic(g: &mut dyn Game, player: PlayerId, source: ObjectId, tapped: bool) -> Result<(), Suspend> {
    if !ask_may_search(g, player, source, if tapped { &FETCH_BASIC_TAPPED } else { &FETCH_BASIC })? {
        return Ok(());
    }
    search_basic_land_to_battlefield(g, player, source, tapped)
}

/// "Search your library for a basic land card, put it onto the battlefield [tapped], then shuffle" (CR 701.23): the search
/// of a Ghost Quarter, a Prismatic Vista and a Lander token (Biotech Specialist), the player finding one or none.
pub fn search_basic_land_to_battlefield(g: &mut dyn Game, player: PlayerId, source: ObjectId, tapped: bool) -> Result<(), Suspend> {
    fetch_basic_land_to_battlefield(g, player, source, tapped)?;
    Ok(())
}

/// "{T}, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle."
/// (CR 701.23, 701.21a): the land's own ability, the sacrifice being a cost. Escape Tunnel, Evolving Wilds and Hobbit Hole print it.
pub const SACRIFICE_TO_FETCH_BASIC_TAPPED: AbilityDef = ability(
    TAP.and(SACRIFICE_THIS),
    |g, item| search_basic_land_to_battlefield(g, item.controller, item.object, true),
    "{T}, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.",
);

/// "Each of these players searches their library for up to N basic land cards, puts them onto the battlefield [tapped], then
/// shuffles", done by the players together (CR 101.4, 701.23i): `searchers` are the players and how many basic lands each
/// looks for, asked in turn order starting with the active player; every land found enters in one event, and only then does
/// each player shuffle. A player who has left the game by their turn to choose does not search (CR 800.4a). Winds of Abandon
/// (the players who lost creatures) and Field of Ruin (each player) print it.
pub fn each_searches_basic_lands_together(g: &mut dyn Game, searchers: &[(PlayerId, u32)], source: ObjectId, tapped: bool) -> Result<(), Suspend> {
    searches_together(g, searchers, source, tapped, None)
}

/// `each_searches_basic_lands_together` for "may search": each player in turn order is asked whether to first and then
/// chooses, all of one player's choices before the next player's (CR 101.4, 101.4c). A player who declines neither searches
/// nor shuffles; the lands found still enter in one event. Krenko's Buzzcrusher (each destroyed land's controller
/// may search, tapped), Old-Growth Dryads (each opponent may search, tapped) and New Frontiers (each player may search,
/// tapped) print it; it is kept beside `each_searches_basic_lands_together` so the order of the players' choices is decided
/// in one place.
pub fn each_may_search_basic_lands_together(g: &mut dyn Game, searchers: &[(PlayerId, u32)], source: ObjectId, tapped: bool) -> Result<(), Suspend> {
    searches_together(g, searchers, source, tapped, Some(if tapped { &FETCH_BASIC_TAPPED } else { &FETCH_BASIC }))
}

fn searches_together(g: &mut dyn Game, searchers: &[(PlayerId, u32)], source: ObjectId, tapped: bool, ask: Option<&'static Question>) -> Result<(), Suspend> {
    let order: Vec<PlayerId> = g.state().apnap().filter(|p| searchers.iter().any(|&(s, _)| s == *p)).collect();
    let destination = if tapped { SearchDestination::BattlefieldTapped } else { SearchDestination::Battlefield };
    let mut lands: Vec<(ObjectId, PlayerId, bool)> = Vec::new();
    let mut searched: Vec<PlayerId> = Vec::new();
    for &player in &order {
        if !crate::players::reached(g, player) {
            continue;
        }
        if let Some(question) = ask {
            if !ask_may_search(g, player, source, question)? {
                continue;
            }
        }
        let count: u32 = searchers.iter().filter(|&&(s, _)| s == player).map(|&(_, n)| n).sum();
        let found = crate::library::search(g, player, player, source, destination, select::basic_land_cards(select::up_to(count)))?;
        lands.extend(found.into_iter().map(|land| (land, player, tapped)));
        searched.push(player);
    }
    g.put_all_onto_battlefield_as(&lands)?;
    for &player in &searched {
        g.shuffle(player);
    }
    Ok(())
}

/// `search_basic_land_to_battlefield`, giving the land that arrived (none if the player found none) for an ability that goes
/// on to do something to it: Elven Passage's "untap that land".
pub fn fetch_basic_land_to_battlefield(g: &mut dyn Game, player: PlayerId, source: ObjectId, tapped: bool) -> Result<Vec<ObjectId>, Suspend> {
    find_to_battlefield(g, player, source, select::BASIC_LAND_CARD, tapped)
}

/// What "you may search your library for a basic land card, reveal it, put it into your hand, then shuffle" asks.
pub const FETCH_BASIC_TO_HAND: Question = Question::new(
    "SearchLibrary",
    "search your library for a basic land card?",
    "Reveal it and put it into your hand, then shuffle",
    "Search",
    "Don't search",
);

/// "Search your library for a basic land card, reveal it, put it into your hand, then shuffle" (CR 701.23): the player
/// finding one or none, and the library shuffled either way. Bushwhack's first mode and Environmental Scientist print it.
pub fn search_basic_land_to_hand(g: &mut dyn Game, player: PlayerId, source: ObjectId) -> Result<(), Suspend> {
    find_to_hand(g, player, source, select::BASIC_LAND_CARD, true)?;
    Ok(())
}

/// "You may search your library for a basic land card, reveal it, put it into your hand, then shuffle": asked first
/// whether to, and declining skips the search and the shuffle (CR 701.23). Environmental Scientist's enters trigger
/// (`search_basic_land_to_hand` is Bushwhack's, which does not say "may"); `ENTERS_MAY_FETCH_BASIC_TO_HAND` below is its whole ability.
pub fn may_search_basic_land_to_hand(g: &mut dyn Game, player: PlayerId, source: ObjectId) -> Result<(), Suspend> {
    if !ask_may_search(g, player, source, &FETCH_BASIC_TO_HAND)? {
        return Ok(());
    }
    search_basic_land_to_hand(g, player, source)
}

/// "When this creature enters, you may search your library for a basic land card, reveal it, put it into your hand, then
/// shuffle." (CR 603.6a, 701.23): the creature's own trigger. Environmental Scientist and Changeling Wayfinder print it.
pub const ENTERS_MAY_FETCH_BASIC_TO_HAND: AbilityDef = triggered(
    triggers::THIS_ENTERS,
    |g, item| may_search_basic_land_to_hand(g, item.controller, item.object),
    "When this creature enters, you may search your library for a basic land card, reveal it, put it into your hand, then shuffle.",
);

/// What "you may search your library for a basic land card, reveal it, then shuffle and put that card on top" asks.
pub const FETCH_BASIC_TO_TOP: Question = Question::new(
    "SearchLibrary",
    "search your library for a basic land card?",
    "Reveal it, then shuffle and put that card on top of your library",
    "Search",
    "Don't search",
);

/// "Search your library for a basic land card, reveal it, then shuffle and put that card on top" (CR 701.23, 701.20): the
/// player finding one or none; the card is revealed while it is still in the library and, the library shuffled, placed on top
/// (it never changes zones). Fountainport Bell and Old Thrush print this exact recipe; it is kept beside its
/// hand and battlefield siblings (`search_basic_land_to_hand`, `search_basic_land_to_battlefield`) so the basic-land predicate and
/// the question wording are the shared ones, and `library::search_to_top` is shared with Enlightened Tutor and Mystical Tutor.
pub fn search_basic_land_to_top(g: &mut dyn Game, player: PlayerId, source: ObjectId) -> Result<(), Suspend> {
    find_to_top(g, player, source, select::BASIC_LAND_CARD, true)?;
    Ok(())
}

/// "You may search your library for a basic land card, reveal it, then shuffle and put that card on top": asked first whether
/// to, and declining skips the search and the shuffle (CR 701.23). Fountainport Bell's and Old Thrush's enters triggers.
pub fn may_search_basic_land_to_top(g: &mut dyn Game, player: PlayerId, source: ObjectId) -> Result<(), Suspend> {
    if !ask_may_search(g, player, source, &FETCH_BASIC_TO_TOP)? {
        return Ok(());
    }
    search_basic_land_to_top(g, player, source)
}

/// "Return a land you control to its owner's hand" (CR 603.6a): the land the controller chooses as the ability resolves, not
/// targeted, asked whenever there is a land to choose; the ability's own source may be the one. The bounce lands return it
/// at once (`bounce_land`); Arid Archway looks at it first, to see whether it is another Desert.
pub fn choose_a_land_to_return(g: &mut dyn Game, item: &StackItem) -> Result<Option<ObjectId>, Suspend> {
    let player = item.controller;
    let lands: Vec<ObjectId> = g.permanents_of(player).into_iter().filter(|&id| g.chars(id).is(CardType::Land)).collect();
    choose::card(g, player, item.object, Purpose::ReturnToHand, lands)
}

fn return_a_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(land) = choose_a_land_to_return(g, item)? {
        g.return_to_hand(&[land]);
    }
    Ok(())
}

/// The bounce lands' (karoos') two abilities, mana ability first: "{T}: Add {A}{B}." (`text` is its full text) and "When
/// this land enters, return a land you control to its owner's hand" (CR 603.6a): chosen as the ability resolves, not
/// targeted, and the land itself may be the one; asked whenever there is a land to choose. With `enters:
/// Some(mana::enters_tapped)` it is Azorius Chancery, Orzhov Basilica and the rest of the cycle.
pub const fn bounce_land(a: Mana, b: Mana, text: &'static str) -> [AbilityDef; 2] {
    [
        mana_ability(TAP, add_each(&[a, b]), text),
        triggered(triggers::THIS_ENTERS, return_a_land, "When this land enters, return a land you control to its owner's hand."),
    ]
}

/// A gain land's two abilities, mana ability last: "When this land enters, you gain 1 life" (CR 603.6a) and "{T}: Add {A} or
/// {B}." (`text` is its full text), the color chosen as it resolves (CR 608.2d). With `enters: Some(mana::enters_tapped)`
/// it is Andoria, Bajor, Wind-Scarred Crag, A.I.M. Labs, Asgardian Citadel, Avengers Hangar, Birnin Zana Plaza, Fisk Tower,
/// Hell's Kitchen, Los Diablos Missile Base, Pym Technologies, Stark Industries and Subterranean Cavern.
pub const fn gain_land(a: Color, b: Color, text: &'static str) -> [AbilityDef; 2] {
    [
        triggered(triggers::THIS_ENTERS, effects::gain_one_life, "When this land enters, you gain 1 life."),
        mana_ability(TAP, add_one_of(&[a, b]), text),
    ]
}

fn fetch_two_types<const A: u16, const B2: u16>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let (a, b) = (word(g, item.object, A), word(g, item.object, B2));
    search_to_battlefield(g, item.controller, item.object, Find::ONE, false, move |q, c| {
        let chars = q.chars(c);
        chars.has_subtype(a) || chars.has_subtype(b)
    })?;
    Ok(())
}

/// A fetch land's ability (CR 701.23): "{T}, Pay 1 life, Sacrifice this land: Search your library for a [type A] or [type B]
/// card, put it onto the battlefield, then shuffle." `text` is the full text. The types are the ones the land's text named
/// as it was sacrificed (CR 612.2, 113.7a). Flooded Strand and Arid Mesa print it.
pub const fn fetch_land<const A: u16, const B2: u16>(text: &'static str) -> AbilityDef {
    ability(TAP.and(life(1)).and(SACRIFICE_THIS), fetch_two_types::<A, B2>, text)
}

fn fetch_basic(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    search_basic_land_to_battlefield(g, item.controller, item.object, false)
}

fn search_three_basic_types<const A: u16, const B2: u16, const C: u16>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let (a, b, c) = (word(g, item.object, A), word(g, item.object, B2), word(g, item.object, C));
    search_to_battlefield(g, item.controller, item.object, Find::ONE, true, move |q, card| {
        let chars = q.chars(card);
        is_basic_land_card(q, card) && (chars.has_subtype(a) || chars.has_subtype(b) || chars.has_subtype(c))
    })?;
    Ok(())
}

/// A landscape's ability (CR 701.23): "{T}, Sacrifice this land: Search your library for a basic [type A], [type B], or
/// [type C] card, put it onto the battlefield tapped, then shuffle." `text` is the full text. A card that is basic (CR
/// 205.4a, 305.6) and has one of the three land types, as the land's text named them when it was sacrificed (CR 612.2,
/// 113.7a). The Landscape cycle prints it: Bountiful, Contaminated, Deceptive, Foreboding, Perilous, Seething, Shattered,
/// Sheltering, Tranquil and Twisted Landscape.
pub const fn fetch_three_basic_types<const A: u16, const B2: u16, const C: u16>(text: &'static str) -> AbilityDef {
    ability(TAP.and(SACRIFICE_THIS), search_three_basic_types::<A, B2, C>, text)
}

/// A fetch land that finds any basic land: "{T}, Pay 1 life, Sacrifice this land: Search your library for a basic land
/// card, put it onto the battlefield, then shuffle." Prismatic Vista prints it, and so does every land that borrows its
/// text.
pub const fn fetch_basic_land(text: &'static str) -> AbilityDef {
    ability(TAP.and(SACRIFICE_THIS).and(life(1)), fetch_basic, text)
}

/// The Hobbit cycle's second ability: "{2}{A}{B}, {T}, Sacrifice this land: Put two +1/+1 counters on target [creature type] you
/// control. Activate only as a sorcery." (CR 602.5d, 122.1): `cost` is "{2}{A}{B}, {T}" and `target` the one target slot (the
/// creature types the land names, read through its words, CR 612). The counters go on the target if it is still legal as the
/// ability resolves, the sacrifice being a cost. Elvenking's Halls, Goblin-town, Iron Hills, Lake-town and Mirkwood print it.
pub const fn sacrifice_for_two_counters(cost_text: &'static str, target: &'static [TargetSpec], text: &'static str) -> AbilityDef {
    ability(cost(cost_text).and(SACRIFICE_THIS), effects::two_plus_one_counters_on_target, text).sorcery_speed().targets(target)
}

/// The triomes' "Cycling {3}" (CR 702.29): with `enters: Some(mana::enters_tapped)` and its three basic land types, a
/// triome. Raffine's Tower and Savai Triome print it.
pub const TRIOME_CYCLING: AbilityDef = triggers::cycling(mana("{3}"), "Cycling {3} ({3}, Discard this card: Draw a card.)");

/// The surveil lands' "When this land enters, surveil 1" (CR 701.25): with `enters: Some(mana::enters_tapped)`, Meticulous
/// Archive and Undercity Sewers.
pub const SURVEIL_LAND: AbilityDef = triggered(triggers::THIS_ENTERS, |g, item| effects::surveil(g, item, 1), "When this land enters, surveil 1.");

/// The Secrets of Strixhaven surveil lands' "{2}{A}{B}, {T}: Surveil 1" (CR 701.25, 602.1): with `enters: Some(mana::enters_tapped)` and
/// its "{T}: Add {A} or {B}.", Fields of Strife, Forum of Amity, Paradox Gardens, Spectacle Summit and Titan's Grave. `cost` and `text`
/// are the card's own.
pub const fn surveil_ability(cost: Cost, text: &'static str) -> AbilityDef {
    ability(cost, |g, item| effects::surveil(g, item, 1), text)
}

/// The scry lands' "When this land enters, scry 1" (CR 701.22): Crystal Grotto (untapped) and, with
/// `enters: Some(mana::enters_tapped)`, Temple of Deceit.
pub const SCRY_LAND: AbilityDef = triggered(triggers::THIS_ENTERS, |g, item| effects::scry(g, item, 1), "When this land enters, scry 1.");

/// "{1}, {T}: Add one mana of any color." (CR 605.1a): the filter ability of Crystal Grotto, Giant's Boulder, Capital City and
/// Conduit Pylons, beside their "{T}: Add {C}.".
pub const ANY_COLOR_FOR_ONE: AbilityDef = mana_ability(cost("{1}, {T}"), add_any_color(1), "{1}, {T}: Add one mana of any color.");

/// A horizon land's mana ability: "{T}, Pay 1 life: Add {A} or {B}." (`text` is its full text), the color named as it is
/// activated (CR 106.5). Horizon Canopy and Fiery Islet print it.
pub const fn pay_life_for_either(a: Color, b: Color, text: &'static str) -> AbilityDef {
    mana_ability(TAP.and(life(1)), add_any_color(1).colors(ColorSet(ColorSet::of(a).0 | ColorSet::of(b).0)), text)
}

/// A filter land's "{A/B}, {T}: Add {A}{A}, {A}{B}, or {B}{B}" (CR 106.1b, 605.3b, 608.2d): two mana, each a color of its
/// own choosing from the two, chosen as the mana is added. `COLORS` is the two colors as a `ColorSet` numbers them, `cost`
/// and `text` the card's own. Every Shadowmoor and Eventide filter land prints it: Fetid Heath, Mystic Gate, Sunken Ruins, Cascade Bluffs,
/// Fire-Lit Thicket, Flooded Grove, Graven Cairns, Rugged Prairie, Twilight Mire and Wooded Bastion.
pub const fn filter_two<const COLORS: u8>(cost: Cost, text: &'static str) -> AbilityDef {
    mana_ability(cost, add_by(add_two_of::<COLORS>), text)
}

fn add_two_of<const COLORS: u8>(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    for color in crate::choose_colors(g, asker.controller, asker.source, ColorSet(COLORS), 2)? {
        g.add_mana(asker.controller, color.mana(), 1);
    }
    Ok(())
}

/// A horizon land's other ability: "{1}, {T}, Sacrifice this land: Draw a card." (`text` is its full text, with the land's
/// name). Horizon Canopy, Fiery Islet and Cryptic Caves (with its own restriction) print it.
pub const fn sacrifice_to_draw(text: &'static str) -> AbilityDef {
    ability(cost("{1}, {T}").and(SACRIFICE_THIS), |g, item| effects::draw(g, item, 1), text)
}

fn controls_either<const A: u16, const B2: u16>(q: &dyn Query, asker: &Asker) -> bool {
    controls_land_type(q, asker.controller, word(q, asker.source, A)) || controls_land_type(q, asker.controller, word(q, asker.source, B2))
}

/// A verge's two mana abilities (CR 602.5b): "{T}: Add {PRIMARY}." and "{T}: Add {SECOND}. Activate only if you control a
/// [type A] or a [type B]", read through the layers, the types as the verge's text names them (CR 612.2). Blazemire Verge and Thornspire Verge print it.
pub const fn verge<const A: u16, const B2: u16>(
    primary: Mana,
    second: Mana,
    primary_text: &'static str,
    second_text: &'static str,
) -> [AbilityDef; 2] {
    [
        mana_ability(TAP, add(primary), primary_text),
        mana_ability(TAP, add(second), second_text).condition(controls_either::<A, B2>),
    ]
}

/// A manland's animation (CR 613.1d, 613.4b): "Until end of turn, this land becomes a P/T [colors] [subtype] creature
/// [with keywords]. It's still a land." The colors and the creature type are the ones the land's own text has now (CR 612),
/// read through its words; the base power and toughness, the keywords and the type are laid over the land (see
/// `animate`). Restless Anchorage, Restless Reef, Restless Vinestalk, Creeping Tar Pit and Celestial Colonnade print it.
pub fn becomes_creature_until_end_of_turn(
    g: &mut dyn Game,
    item: &StackItem,
    (power, toughness): (i16, i16),
    colors: ColorSet,
    subtype: Subtype,
    keywords: Keywords,
) {
    let words = g.words(item.object);
    let animation = crate::Animation {
        power,
        toughness,
        colors: Some(words.colors(colors)),
        types: TypeSet::NONE,
        subtypes: &[words.subtype(subtype)],
        all_creature_types: false,
        replaces_creature_types: false,
        keywords,
        ability: None,
    };
    crate::animate(g, item.object, item.object, &animation, Duration::EndOfTurn);
}

/// What "[player] may search their library for a basic land card, put it onto the battlefield, then shuffle" asks.
pub const FETCH_BASIC: Question = Question::new(
    "SearchLibrary",
    "search your library for a basic land card?",
    "It goes onto the battlefield, then you shuffle",
    "Search",
    "Don't search",
);

/// "Activate only if this land entered this turn or if you control a basic land" (CR 602.5b): checked as the ability is
/// activated. Entering is asked of the event history (CR 400.7: a land that left and came back is a new object that entered
/// this turn); a basic land is a land with the basic supertype (CR 205.4a), read through the layers. The land itself is not
/// basic, so it never counts itself. Gleaming Bastion, Training Compound, Hidden Lair and Gathering Place print it.
pub fn entered_this_turn_or_you_control_a_basic_land(q: &dyn Query, asker: &Asker) -> bool {
    crate::history::entered_this_turn(q.history(), asker.source)
        || q.permanents_of(asker.controller).into_iter().any(|id| {
            let chars = q.chars(id);
            chars.is(CardType::Land) && chars.supertypes.basic
        })
}

/// The land's two mana abilities: "{T}: Add {C}." and "{T}: Add {A} or {B}. Activate only if this land entered this turn or
/// if you control a basic land." (`text` is the second's full text). Gleaming Bastion, Training Compound, Hidden Lair,
/// Gathering Place and Dark Fortress print them.
pub const fn basic_land_dual(a: Color, b: Color, text: &'static str) -> [AbilityDef; 2] {
    [
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        mana_ability(TAP, add_one_of(&[a, b]), text).condition(entered_this_turn_or_you_control_a_basic_land),
    ]
}

/// The same search, the land put onto the battlefield tapped (Path to Exile, Erode).
pub const FETCH_BASIC_TAPPED: Question = Question::new(
    "SearchLibrary",
    "search your library for a basic land card?",
    "It enters tapped, then you shuffle",
    "Search",
    "Don't search",
);

/// "The number of [lands] you control", the value of a characteristic-defining ability (CR 604.3, 613.1): the permanents on the
/// battlefield `counts` accepts, read through the layers before the one being applied (so a land made an Island in layer 4 is
/// counted by a layer 7a ability). "You" is the controller of `source` (CR 108.4: the engine keeps the owner as controller off the
/// battlefield and stack, CR 108.4a, and the caster on the stack): Lumbering Worldwagon's power and Sandman's power and toughness ("works in all zones")
/// count lands; `counts` is `|c| c.is(CardType::Land)` for them, and Desert Were-Worm's "for each Mountain you control" counts the permanents with the Mountain
/// subtype.
pub fn count_you_control(q: &dyn LayerQuery, source: ObjectId, counts: &dyn Fn(&Chars) -> bool) -> i16 {
    let state = q.state();
    let obj = state.obj(source);
    state.battlefield().iter().filter(|&&id| state.obj(id).controller == obj.controller && counts(&q.chars(id))).count() as i16
}

/// "The number of differently named lands you control" (CR 201.2a-b): count the largest set of lands whose names do not
/// overlap, including split cards and Rooms with multiple names, so the count is the true largest set whatever the order of the
/// lands. Emil, Vastlands Roamer, Astute Terraformer, Branching Timeline, Cloak and Phaser and Deep Space Scan print it.
pub fn differently_named_lands(q: &dyn Query, player: PlayerId) -> usize {
    let names: Vec<Vec<&'static str>> = q
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| q.obj(id).controller == player && q.chars(id).is(CardType::Land))
        .map(|id| crate::library::object_names(q, id))
        .collect();
    crate::select::largest_name_set(&names).len()
}

/// "[This creature]'s power and toughness are each equal to the number of lands you control" as the `StaticDef::apply` of the
/// ability (CR 613.4a for a characteristic-defining one, CR 604.3; layer 7b for one an effect gave, CR 613.4b), read live
/// (CR 611.3a). "You" is the controller of `source` (see `count_you_control`). Beorn's Hospitality (given), Lumra, Bellow of the
/// Woods, Mirkwood Pathmaker and Sandman, Shifting Scoundrel print it.
pub fn power_and_toughness_are_lands_you_control(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let lands = count_you_control(q, source, &|c| c.is(CardType::Land));
    chars.power = Some(lands);
    chars.toughness = Some(lands);
}

/// "Return all land cards from your graveyard to the battlefield tapped" (Aftermath Analyst, Lumra, Bellow of the Woods,
/// Splendid Reclamation): every land card in `player`'s graveyard as this is done, put onto the battlefield together
/// (CR 603.6a), tapped (CR 110.5b), under that player's control (CR 110.2a). Only that player's own graveyard is looked at.
pub fn return_all_land_cards_from_graveyard_tapped(g: &mut dyn Game, player: PlayerId) -> Result<(), Suspend> {
    let lands: Vec<(ObjectId, PlayerId, bool)> = g
        .state()
        .zone(Zone::graveyard(player))
        .iter()
        .copied()
        .filter(|&card| g.chars(card).is(CardType::Land))
        .map(|card| (card, player, true))
        .collect();
    g.put_all_onto_battlefield_as(&lands)?;
    Ok(())
}
