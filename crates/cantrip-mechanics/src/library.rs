//! Library recipes (CR 401, 701.23): searching, looking at the top, exiling it to play.
//!
//! **A search is `search`, or one of the `find_to_*` wrappers, with a rule from `crate::select`** ("a basic land card",
//! "up to two land cards", "cards with different names"). `Find` and the `search_to_*` wrappers, which take a count and a
//! closure, are kept for existing cards; new cards use the rules.

use crate::{search_library, top_of_library};
use cantrip_api::*;

/// The cards of `owner`'s library as a search shows them: bottom first, as the C# engine lists them.
pub fn searched_cards(q: &dyn Query, owner: PlayerId) -> Vec<ObjectId> {
    let mut library: Vec<ObjectId> = q.state().zone(Zone::library(owner)).to_vec();
    library.reverse();
    library
}

/// `searcher` searches the library of `owner` (CR 701.23, 701.23f; the same player for "search your library") for the
/// cards `rule` allows (`crate::select`: `select::CARD`, `select::basic_land_cards(up_to(2))`), and they come back in the
/// order chosen. The whole library is shown, and the searcher is told the cards found go to `destination`. Asked only when
/// the library has cards and the rule could take one; the search is recorded either way (a search for zero cards, as
/// Settle the Wreckage can make, is still a search). A search the spell or ability can't cause (`Game::may_search`,
/// CR 101.2) is not made: nothing is asked or found and no search is recorded, and the caller's "then shuffle" still
/// shuffles (Ashiok, Dream Render's ruling). What becomes of the cards, and the shuffle, are the caller's.
pub fn search(g: &mut dyn Game, searcher: PlayerId, owner: PlayerId, source: ObjectId, destination: SearchDestination, rule: Selection) -> Result<Vec<ObjectId>, Suspend> {
    search_with(g, searcher, owner, source, destination, rule, Vec::new())
}

/// `search`, the cards `also` shown with the library's and chosen from with them: a search of other zones in the same
/// instruction (CR 701.23a), such as the searcher's own hand (`search_hand_and_or_library`).
fn search_with(g: &mut dyn Game, searcher: PlayerId, owner: PlayerId, source: ObjectId, destination: SearchDestination, rule: Selection, also: Vec<ObjectId>) -> Result<Vec<ObjectId>, Suspend> {
    if !g.may_search(searcher, owner) {
        return Ok(Vec::new());
    }
    let library: Vec<ObjectId> = also.into_iter().chain(searched_cards(g, owner)).collect();
    let found = if library.is_empty() || rule.count.most() == Some(0) {
        Vec::new()
    } else {
        match g.ask(searcher, RequestKind::SearchLibrary { source, library, rule: Box::new(rule), destination })? {
            Answer::Cards(cards) => cards,
            _ => Vec::new(),
        }
    };
    g.searched(searcher, owner);
    Ok(found)
}

/// `player` searches their own library for the cards `rule` allows, revealing them with `reveal` ("reveal it").
fn find_by(g: &mut dyn Game, player: PlayerId, source: ObjectId, destination: SearchDestination, rule: Selection, reveal: bool) -> Result<Vec<ObjectId>, Suspend> {
    let found = search(g, player, player, source, destination, rule)?;
    if reveal {
        g.reveal(player, &found);
    }
    Ok(found)
}

/// "Search your library for [`rule`], [reveal it,] put it into your hand, then shuffle" (CR 701.23): the cards found, as
/// they are in the hand. Demonic Tutor (`select::CARD`) and the basic land searches (`select::BASIC_LAND_CARD`, revealed)
/// print it.
pub fn find_to_hand(g: &mut dyn Game, player: PlayerId, source: ObjectId, rule: Selection, reveal: bool) -> Result<Vec<ObjectId>, Suspend> {
    let found = find_by(g, player, source, SearchDestination::Hand, rule, reveal)?;
    Ok(into_hand(g, player, &found, reveal))
}

/// "Search your library for [`rule`], put it onto the battlefield [tapped], then shuffle" (CR 701.23): the permanents that
/// arrived. The basic land fetches (`select::BASIC_LAND_CARD`) and Crop Rotation (`select::LAND_CARD`) print it.
pub fn find_to_battlefield(g: &mut dyn Game, player: PlayerId, source: ObjectId, rule: Selection, tapped: bool) -> Result<Vec<ObjectId>, Suspend> {
    let to = if tapped { SearchDestination::BattlefieldTapped } else { SearchDestination::Battlefield };
    let found = find_by(g, player, source, to, rule, false)?;
    onto_battlefield(g, player, found, tapped)
}

/// "Search your library for [`rule`], put it into your graveyard, then shuffle" (CR 701.23). Entomb (`select::CARD`) and
/// Goblin Engineer (`select::ARTIFACT_CARD`) print it.
pub fn find_to_graveyard(g: &mut dyn Game, player: PlayerId, source: ObjectId, rule: Selection) -> Result<Vec<ObjectId>, Suspend> {
    let found = find_by(g, player, source, SearchDestination::Graveyard, rule, false)?;
    Ok(into_graveyard(g, player, &found))
}

/// "Search your library for [`rule`], [reveal it,] then shuffle and put that card on top" (CR 701.23). Vampiric Tutor
/// (`select::CARD`) and Worldly Tutor (`select::CREATURE_CARD`, revealed) print it.
pub fn find_to_top(g: &mut dyn Game, player: PlayerId, source: ObjectId, rule: Selection, reveal: bool) -> Result<Vec<ObjectId>, Suspend> {
    let found = find_by(g, player, source, SearchDestination::TopOfLibrary, rule, reveal)?;
    onto_top(g, player, &found);
    Ok(found)
}

/// "Search your hand and/or library for [`rule`]" (CR 701.23, 701.23a-b): the cards found, and whether the library was
/// searched, for the "if you search your library this way, shuffle" that follows. The player chooses first whether to search
/// the library (`ask`, by `ask_may_search`, a search the effect can't cause being declined). If they do, the hand and the
/// library are searched together: one selection over the cards of both that `rule` allows, so the one card of "a [quality]
/// card" may be taken from either, having seen both (the library is searched whatever is found, and the caller shuffles it). If
/// they don't, only the hand is searched, and nothing is asked when no card there fits. A hand is no hidden zone to its owner, so
/// searching it asks no question of its own. The cards are left where they were found for the caller to move.
/// Last Light of Durin's Day (a Dragon card, onto the battlefield) and Aether Searcher (a card with a noted name, cast) print it.
pub fn search_hand_and_or_library(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    destination: SearchDestination,
    purpose: Purpose,
    ask: &'static Question,
    rule: Selection,
) -> Result<(Vec<ObjectId>, bool), Suspend> {
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    if ask_may_search(g, player, source, ask)? {
        let found = search_with(g, player, player, source, destination, rule, hand)?;
        return Ok((found, true));
    }
    let any = cantrip_api::select::first_answer(&*g, &rule, player, source, &hand, true).is_some_and(|taken| !taken.is_empty());
    let found = if any { crate::choose::select(g, player, source, purpose, hand, rule)? } else { Vec::new() };
    Ok((found, false))
}

/// The cards found put into the hand, by the instruction that reveals them when the search says "reveal it" (CR 701.20a:
/// `Game::move_together_revealed`).
fn into_hand(g: &mut dyn Game, player: PlayerId, found: &[ObjectId], revealed: bool) -> Vec<ObjectId> {
    let moves: Vec<(ObjectId, Zone)> = found.iter().map(|&c| (c, Zone::hand(player))).collect();
    let moved = if revealed { g.move_together_revealed(&moves, MoveReason::Search) } else { g.move_together(&moves, MoveReason::Search) };
    g.shuffle(player);
    moved
}

fn into_graveyard(g: &mut dyn Game, player: PlayerId, found: &[ObjectId]) -> Vec<ObjectId> {
    let moves: Vec<(ObjectId, Zone)> = found.iter().map(|&c| (c, Zone::graveyard(player))).collect();
    let moved = g.move_together(&moves, MoveReason::Search);
    g.shuffle(player);
    moved
}

fn onto_battlefield(g: &mut dyn Game, player: PlayerId, found: Vec<ObjectId>, tapped: bool) -> Result<Vec<ObjectId>, Suspend> {
    // All the cards found enter at the same time (CR 110.2a, 701.23): each sees the others already there, as a landfall-like
    // "if you control N other Forests" does (Roiling Canopy).
    let arrived = g.put_all_onto_battlefield_as(&found.into_iter().map(|c| (c, player, tapped)).collect::<Vec<_>>())?;
    g.shuffle(player);
    Ok(arrived)
}

fn onto_top(g: &mut dyn Game, player: PlayerId, found: &[ObjectId]) {
    g.shuffle(player);
    if !found.is_empty() {
        g.arrange_library(player, found, &[]);
    }
}

/// What a library search looks for (CR 701.23): how many cards, whether finding fewer is allowed, whether the cards
/// found are revealed. `Find::ONE` is "a [quality] card", which a player may fail to find (CR 701.23b); `Find::ANY` is
/// "a card" with nothing to fail at, so the searcher must take one. Kept for existing cards; new cards say what they look
/// for with a rule from `crate::select` and search with `search` or the `find_to_*` wrappers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Find {
    pub count: u32,
    pub optional: bool,
    pub reveal: bool,
}

impl Find {
    /// "Search your library for a [quality] card": one card, which may not be found.
    pub const ONE: Find = Find { count: 1, optional: true, reveal: false };
    /// "Search your library for a card": one card, taken if there is any.
    pub const ANY: Find = Find { count: 1, optional: false, reveal: false };
    /// The cards found are revealed ("reveal it").
    pub const fn revealed(self) -> Find {
        Find { reveal: true, ..self }
    }
    /// Up to `count` cards instead of one.
    pub const fn up_to(self, count: u32) -> Find {
        Find { count, ..self }
    }
}

fn find(g: &mut dyn Game, player: PlayerId, source: ObjectId, destination: SearchDestination, find: Find, eligible: impl Fn(&dyn Query, ObjectId) -> bool) -> Result<Vec<ObjectId>, Suspend> {
    let found = search_library(g, player, player, source, destination, find.count, find.optional, eligible)?;
    if find.reveal {
        g.reveal(player, &found);
    }
    Ok(found)
}

/// "[Player] may search their library for [cards] ..." (CR 701.23, 101.2): whether `player` searches, asked with `question`
/// (verb `SearchLibrary`, the search worded). Not asked, and false, when the spell or ability can't cause the search
/// (`Game::may_search`: Ashiok, Dream Render's ruling, "your opponents can't choose to search, so they won't shuffle").
/// Every "may search" is asked here, and the search recipe (with its "then shuffle") runs only on true, so a declined or
/// forbidden search shuffles nothing. Stoneforge Mystic, Trinket Mage, Golos, Tireless Pilgrim and the basic-land fetches of
/// Path to Exile and Ghost Quarter (`lands::may_fetch_basic`) ask it.
pub fn ask_may_search(g: &mut dyn Game, player: PlayerId, source: ObjectId, question: &'static Question) -> Result<bool, Suspend> {
    if !g.may_search(player, player) {
        return Ok(false);
    }
    crate::effects::ask_may(g, player, source, Purpose::Ask(question))
}

/// "Search [a player's] library for [cards] and exile them, then that player shuffles" (CR 701.23, 406.2):
/// `searcher` searches the library of `owner` (the same player, or another), and the cards found, together with `also`
/// (cards the same search found in other zones, such as the graveyard and hand: CR 701.23a lets a search span zones),
/// go to exile at once, `also` first, as an exile (`ZoneReason::Exile`): the search only finds the cards, and "exile
/// them" is the exile. `also` exists so that the cards found in other zones leave in the same simultaneous move as the
/// library finds, not one move before or after. Each card goes to its owner's exile. The new objects, in that order, so
/// a caller can tell which of `also` were exiled. Deadly Cover-Up searches another player's
/// library and exiles what it finds with the graveyard and hand cards of the same search; Ugin, Eye of the Storms' -11
/// searches its controller's own.
pub fn search_to_exile(
    g: &mut dyn Game,
    searcher: PlayerId,
    owner: PlayerId,
    source: ObjectId,
    how: Find,
    also: &[ObjectId],
    eligible: impl Fn(&dyn Query, ObjectId) -> bool,
) -> Result<Vec<ObjectId>, Suspend> {
    let found = search_library(g, searcher, owner, source, SearchDestination::Exile, how.count, how.optional, eligible)?;
    if how.reveal {
        g.reveal(searcher, &found);
    }
    let moves: Vec<(ObjectId, Zone)> = also.iter().chain(found.iter()).map(|&c| (c, Zone::exile(g.obj(c).owner))).collect();
    let moved = g.move_together(&moves, MoveReason::Exile);
    g.shuffle(owner);
    Ok(moved)
}

/// "Search [`owner`]'s graveyard, hand, and library for any number of cards with the same name as" an object whose `names` the
/// caller read before it moved (`object_names`: CR 608.2h, the object as it last existed, since a move makes a new object,
/// CR 400.7, and a card's copy, face-down or transformed state is gone with it) "and exile them.
/// That player shuffles, then draws a card for each card exiled from their hand this way" (CR 701.23, 701.23a, 406.2): the
/// search of `exile_any_number_named_from_graveyard_hand_and_library`, then `owner` draws by its count. Deadly Cover-Up (the
/// owner of a card it exiled), The End (the controller of the permanent it exiled) and Test of Talents (the controller of the
/// spell it countered) print it.
pub fn exile_same_name_from_graveyard_hand_and_library(g: &mut dyn Game, searcher: PlayerId, owner: PlayerId, source: ObjectId, names: &[&'static str]) -> Result<(), Suspend> {
    let from_hand = exile_any_number_named_from_graveyard_hand_and_library(g, searcher, owner, source, names)?;
    g.draw(owner, from_hand);
    Ok(())
}

/// "Search [`owner`]'s graveyard, hand, and library for any number of cards with [one of `names`] and exile them. That player
/// shuffles": how many of the cards found reached exile from the hand, for what "for each card exiled from their hand this
/// way" gives after the shuffle. `searcher` (the spell's controller) does the searching, in the order the text names the
/// zones; each zone is a search for any number (CR 701.23b), so cards of the name may be left. A card matches by sharing one
/// of `names` (CR 201.2); the cards found go to exile together and `owner` shuffles (CR 701.23, 406.2), and a card a
/// replacement sent elsewhere is not counted. Necromentia (a token for each), Deadly Cover-Up, The End and Test of Talents (a
/// card drawn for each: `exile_same_name_from_graveyard_hand_and_library`) print it.
pub fn exile_any_number_named_from_graveyard_hand_and_library(
    g: &mut dyn Game,
    searcher: PlayerId,
    owner: PlayerId,
    source: ObjectId,
    names: &[&'static str],
) -> Result<u32, Suspend> {
    let has_name = |q: &dyn Query, c: ObjectId| object_names(q, c).iter().any(|n| names.contains(n));
    let graveyard: Vec<ObjectId> = g.state().zone(Zone::graveyard(owner)).iter().copied().filter(|&c| has_name(g, c)).collect();
    let mut found = crate::choose::any_number_of(g, searcher, source, Purpose::Exile, "", graveyard.clone(), graveyard)?;
    let in_graveyard = found.len();
    let hand: Vec<ObjectId> = g.state().zone(Zone::hand(owner)).iter().copied().filter(|&c| has_name(g, c)).collect();
    found.extend(crate::choose::any_number_of(g, searcher, source, Purpose::Exile, "", hand.clone(), hand)?);
    let cards_found = found.len();
    // The library is searched and everything found goes to exile together; the owner shuffles (CR 701.23, 406.2).
    let moved = search_to_exile(g, searcher, owner, source, Find::ONE.up_to(crate::ANY_NUMBER), &found, |q, c| has_name(q, c))?;
    // Only one that actually reached exile from the hand counts: a replacement may have sent one elsewhere.
    Ok(moved[in_graveyard..cards_found].iter().filter(|&&c| g.obj(c).zone.is(ZoneKind::Exile)).count() as u32)
}

/// "Choose a card name. Search target opponent's graveyard, hand, and library for up to four cards with that name and exile
/// them. Then that player shuffles." - the effect of the resolving `item`, whose first target is the opponent. The name is
/// chosen as it resolves, not as it was put on the stack (CR 608.2c), and may be any card name, a basic land's included
/// (CR 201.4a). The controller searches the three zones in the order the text names them, choosing among the cards with the
/// name in each, in all up to four (a player need not take them all, CR 701.23b); everything found is exiled at once and the
/// owner shuffles (CR 701.23, 406.2). An opponent no longer a legal target means nothing happens (CR 608.2b). Ancient
/// Vendetta and The Rise of Sozin's chapter II print it.
pub fn exile_up_to_four_named_from_graveyard_hand_and_library(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_up_to_four_named(g, item)?;
    Ok(())
}

/// "Choose a card name. Search target opponent's graveyard, hand, and library for up to four cards with that name and exile
/// them. That player shuffles, then draws a card for each card exiled from their hand this way" (CR 701.23, 406.2): the
/// search of `exile_up_to_four_named_from_graveyard_hand_and_library`, then the owner draws one card for each found card that
/// actually reached exile from their hand (a replacement may have sent one elsewhere). The Stone Brain prints it; Deadly
/// Cover-Up and The End draw by the same count (`exile_same_name_from_graveyard_hand_and_library`).
pub fn exile_up_to_four_named_and_draw_for_each_from_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some((owner, from_hand)) = exile_up_to_four_named(g, item)? {
        g.draw(owner, from_hand);
    }
    Ok(())
}

/// The search both make: the owner searched and how many cards reached exile from their hand, or `None` when the target
/// opponent is no longer a legal target (CR 608.2b).
fn exile_up_to_four_named(g: &mut dyn Game, item: &StackItem) -> Result<Option<(PlayerId, u32)>, Suspend> {
    // Up to this many cards in all, whichever zones they are found in.
    const MOST: u8 = 4;
    let you = item.controller;
    let Some(owner) = crate::targets::player(g, item, 0) else { return Ok(None) };
    let name = g.ask(you, RequestKind::ChooseName { source: item.object, looked_at: Vec::new(), suggestion: None, kind: NameKind::ANY })?;
    // A card has "that name" when it is the card name chosen; with no name chosen nothing has it.
    let named = |q: &dyn Query, c: ObjectId| matches!(name, Answer::Name(Some(wanted)) if object_has_name(q, c, wanted));
    let graveyard: Vec<ObjectId> = g.state().zone(Zone::graveyard(owner)).iter().copied().filter(|&c| named(g, c)).collect();
    let mut found = crate::choose::cards(g, you, item.object, Purpose::Exile, graveyard, u32::from(MOST), true)?;
    let in_graveyard = found.len();
    let hand: Vec<ObjectId> = g.state().zone(Zone::hand(owner)).iter().copied().filter(|&c| named(g, c)).collect();
    let remaining = MOST - found.len() as u8;
    found.extend(crate::choose::cards(g, you, item.object, Purpose::Exile, hand, u32::from(remaining), true)?);
    let cards_found = found.len();
    let remaining = MOST - found.len() as u8;
    let moved = search_to_exile(g, you, owner, item.object, Find::ONE.up_to(u32::from(remaining)), &found, named)?;
    let from_hand = moved[in_graveyard..cards_found].iter().filter(|&&c| g.obj(c).zone.is(ZoneKind::Exile)).count() as u32;
    Ok(Some((owner, from_hand)))
}

/// "Search your library for [cards], put them into your hand, then shuffle" (CR 701.23): the cards found, as they are
/// in the hand. Demonic Tutor (`Find::ANY`) and Expedition Map (`Find::ONE.revealed()`) print it. Kept for existing cards; new cards
/// use the `find_to_*` wrapper with a rule from `crate::select`.
pub fn search_to_hand(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    how: Find,
    eligible: impl Fn(&dyn Query, ObjectId) -> bool,
) -> Result<Vec<ObjectId>, Suspend> {
    let found = find(g, player, source, SearchDestination::Hand, how, eligible)?;
    Ok(into_hand(g, player, &found, how.reveal))
}

/// "Search your library for an Aura or Equipment card, reveal it, put it into your hand, then shuffle" (CR 701.23). Open the
/// Armory and Birthright Boon (Kellan, the Fae-Blooded's Adventure) print it.
pub fn search_aura_or_equipment_to_hand(g: &mut dyn Game, player: PlayerId, source: ObjectId) -> Result<Vec<ObjectId>, Suspend> {
    search_to_hand(g, player, source, Find::ONE.revealed(), |q, c| {
        let chars = q.chars(c);
        chars.has_subtype(Subtype::Aura) || chars.has_subtype(Subtype::Equipment)
    })
}

/// "Search your library for [cards], put them into your graveyard, then shuffle" (CR 701.23). Entomb and Goblin
/// Engineer print it. Kept for existing cards; new cards
/// use the `find_to_*` wrapper with a rule from `crate::select`.
pub fn search_to_graveyard(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    how: Find,
    eligible: impl Fn(&dyn Query, ObjectId) -> bool,
) -> Result<Vec<ObjectId>, Suspend> {
    let found = find(g, player, source, SearchDestination::Graveyard, how, eligible)?;
    Ok(into_graveyard(g, player, &found))
}

/// "Search your library for [cards], put them onto the battlefield [tapped], then shuffle" (CR 701.23): the permanents
/// that arrived. The fetch lands (Flooded Strand, Arid Mesa), Tinker and Natural Order print it. Kept for existing cards; new cards
/// use the `find_to_*` wrapper with a rule from `crate::select`.
pub fn search_to_battlefield(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    how: Find,
    tapped: bool,
    eligible: impl Fn(&dyn Query, ObjectId) -> bool,
) -> Result<Vec<ObjectId>, Suspend> {
    let found = find(g, player, source, if tapped { SearchDestination::BattlefieldTapped } else { SearchDestination::Battlefield }, how, eligible)?;
    onto_battlefield(g, player, found, tapped)
}

/// "Search your library for [a card] and reveal it. Put it onto the battlefield tapped if it's a land card. Otherwise, put
/// it into your hand. Then shuffle." (CR 701.23, 701.20, 305.4): `how` is one card (`Find::ONE`, which may go unfound,
/// CR 701.23b; `.revealed()` for "and reveal it"), and the search tells the searcher where each kind of card goes
/// (`SearchDestination::LandTappedElseHand`). Whether it is a land card is read as it is in the library, where a
/// double-faced card has only its front face's characteristics (CR 712.8a). A land is put onto the battlefield tapped
/// under the searcher's control, which is not playing it (CR 305.4); any other card goes to the hand. The library is
/// shuffled whether or not a card was found. The card found, where it went. Archdruid's Charm's first mode prints it: it
/// is the library-search form of the split Nadu, Winged Wisdom and Nissa, Sage Animist make with the top card of the
/// library (untapped, after no search), and Treasure Chest's 20 makes for an artifact card with a "may".
pub fn search_land_tapped_else_hand(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    how: Find,
    eligible: impl Fn(&dyn Query, ObjectId) -> bool,
) -> Result<Option<ObjectId>, Suspend> {
    debug_assert_eq!(how.count, 1, "the search finds a card: one card goes one way or the other");
    let found = find(g, player, source, SearchDestination::LandTappedElseHand, how, eligible)?;
    let went = match found.first() {
        Some(&land) if g.chars(land).is(CardType::Land) => Some(g.put_onto_battlefield(land, player, true)?),
        Some(&card) if how.reveal => g.move_together_revealed(&[(card, Zone::hand(player))], MoveReason::Search).first().copied(),
        Some(&card) => g.move_together(&[(card, Zone::hand(player))], MoveReason::Search).first().copied(),
        None => None,
    };
    g.shuffle(player);
    Ok(went)
}

/// "Search your library for [a card], then shuffle and put that card on top" (CR 701.23): the library is shuffled
/// and the card placed on top; it never changes zones. Enlightened Tutor and Vampiric Tutor print it. Kept for existing cards; new cards
/// use the `find_to_*` wrapper with a rule from `crate::select`.
pub fn search_to_top(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    how: Find,
    eligible: impl Fn(&dyn Query, ObjectId) -> bool,
) -> Result<Vec<ObjectId>, Suspend> {
    let found = find(g, player, source, SearchDestination::TopOfLibrary, how, eligible)?;
    onto_top(g, player, &found);
    Ok(found)
}

/// What becomes of the cards looked at and not taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rest {
    /// On the bottom of the library in a random order (CR 401.4): nobody is asked for an order.
    BottomRandom,
    /// On the bottom in any order the player chooses, the first highest of them and the last lowest.
    BottomChosen,
}

/// "Look at the top `count` cards of your library. Put between `min` and `max` of them that pass `wanted` into your hand
/// and the rest on the bottom of your library" (CR 401.2, 701.20-ish "look"): `reveal` makes it "you may reveal". A
/// library with fewer cards shows what it has (CR 609.3), and `min` and `max` shrink to the cards that qualify. With
/// nothing qualifying the player is still asked, to see the cards (the look is the instruction, the taking only its
/// option), and the hand takes none. `wanted` may capture the words of the source's text (CR 612). The cards taken, as they are in the hand. Once Upon a Time and Narset, Parter of
/// Veils (one, revealed, random), Dig Through Time and Stock Up (two, chosen order) print it.
pub fn look_at_top(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    count: usize,
    (min, max): (u8, u8),
    reveal: bool,
    wanted: impl Fn(&Chars) -> bool,
    rest: Rest,
) -> Result<Vec<ObjectId>, Suspend> {
    let looked = top_of_library(g, player, count);
    if looked.is_empty() {
        return Ok(Vec::new());
    }
    g.look_at(player, &looked);
    let takeable: Vec<ObjectId> = looked.iter().copied().filter(|&c| wanted(&g.chars(c))).collect();
    // At most the cards that qualify, saturating: past 255 of them a bound of 255 (`u8::MAX`, "any number") stays one.
    let qualifying = u8::try_from(takeable.len()).unwrap_or(u8::MAX);
    let min = min.min(qualifying);
    let max = max.min(qualifying);
    let ask = RequestKind::DistributeCards {
        source,
        cards: looked.clone(),
        destinations: vec![
            Destination { kind: DestinationKind::Hand, min, max, eligible: takeable },
            Destination { kind: DestinationKind::LibraryBottom, min: 0, max: u8::MAX, eligible: looked.clone() },
        ],
        may_shuffle: false,
    };
    let (taken, bottom) = match g.ask(player, ask)? {
        Answer::Placements { placements, .. } => {
            (placements.first().cloned().unwrap_or_default(), placements.get(1).cloned().unwrap_or_default())
        }
        _ => (Vec::new(), Vec::new()),
    };
    let taken = if taken.is_empty() {
        taken
    } else {
        if reveal {
            g.reveal(player, &taken);
            g.return_to_hand_revealed(&taken)
        } else {
            g.return_to_hand(&taken)
        }
    };
    match rest {
        Rest::BottomChosen => g.arrange_library(player, &[], &bottom),
        Rest::BottomRandom => rest_to_bottom_in_random_order(g, player, &looked),
    }
    Ok(taken)
}

/// "Look at the top `count` cards of your library. Put up to `most` [`kind`] cards [with total mana value N or less]
/// from among them onto the battlefield. Put the rest on the bottom of your library in a random order [or
/// `Rest::BottomChosen`: "in any order"]." (CR 401.2, 401.4): the player sees every card looked at (a smaller library
/// shows what it has, CR 609.3) and chooses among those `wanted` says qualify, read where they are (an {X} in the
/// library is 0, CR 202.3e); with `total`, the cards chosen together have mana values adding up to at most it
/// (`choose::cards_within_total`), without it each card is judged alone. The cards chosen enter the battlefield at
/// once, under the player's control (CR 110.2a), `tapped` or not, and the others still in the library go to the bottom, as `rest` says.
/// With nothing qualifying nobody is asked. The permanents that arrived are returned. Aang, at the Crossroads ("you may
/// put a creature card with mana value 4 or less", one), United Battlefront and Storm the Festival (up to two), and
/// Michelangelo's Technique (up to two, total mana value 6 or less) print it with a random order, untapped; Kaslem's
/// Stonetree, Silverback Elder, Elvish Rejuvenator, Ignis Scientia and Freestrider Lookout (one land card, `tapped`) too; Collected Company (up
/// to two) and Mayael the Anima (one) with the player's.
pub fn look_at_top_onto_battlefield(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    count: usize,
    most: u8,
    kind: &'static str,
    wanted: impl Fn(&dyn Query, ObjectId) -> bool,
    total: Option<u32>,
    tapped: bool,
    rest: Rest,
) -> Result<Vec<ObjectId>, Suspend> {
    look_at_top_onto_battlefield_then(g, player, source, count, most, kind, wanted, total, tapped, rest, |_, _| Ok(()))
}

/// `look_at_top_onto_battlefield` with a step between the cards entering and the rest going to the bottom: "Put a Hero,
/// Equipment, or Vehicle card from among them onto the battlefield. If it's a double-faced card, you may transform it. Put the
/// rest on the bottom of your library in a random order" (CR 608.2c). `then` is handed the permanents that arrived and runs
/// before the rest are placed. Nick Fury, Agent of S.H.I.E.L.D. prints it.
pub fn look_at_top_onto_battlefield_then(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    count: usize,
    most: u8,
    kind: &'static str,
    wanted: impl Fn(&dyn Query, ObjectId) -> bool,
    total: Option<u32>,
    tapped: bool,
    rest: Rest,
    then: impl FnOnce(&mut dyn Game, &[ObjectId]) -> Result<(), Suspend>,
) -> Result<Vec<ObjectId>, Suspend> {
    let looked = top_of_library(g, player, count);
    if looked.is_empty() {
        return Ok(Vec::new());
    }
    g.look_at(player, &looked);
    let permitted: Vec<ObjectId> = looked.iter().copied().filter(|&c| wanted(g, c)).collect();
    let chosen = match total {
        Some(total) => crate::choose::cards_within_total(g, player, source, Purpose::PutOntoBattlefield, kind, looked.clone(), permitted, u32::from(most), total)?,
        None => crate::choose::cards_from_kind(g, player, source, Purpose::PutOntoBattlefield, kind, looked.clone(), permitted, u32::from(most), true)?,
    };
    let arrived = if chosen.is_empty() {
        Vec::new()
    } else {
        let entering: Vec<(ObjectId, PlayerId, bool)> = chosen.into_iter().map(|c| (c, player, tapped)).collect();
        g.put_all_onto_battlefield_as(&entering)?
    };
    then(g, &arrived)?;
    match rest {
        Rest::BottomRandom => rest_to_bottom_in_random_order(g, player, &looked),
        Rest::BottomChosen => {
            let left: Vec<ObjectId> = looked.iter().copied().filter(|&c| is_live(g, c) && g.obj(c).zone == Zone::library(player)).collect();
            let order = ask_library_order(g, player, source, &left, DestinationKind::LibraryBottom)?;
            g.arrange_library(player, &[], &order);
        }
    }
    Ok(arrived)
}

/// "Put the rest on the bottom of your library in a random order" (CR 401.4) after looking at `looked`, cards of `player`'s
/// library: those still in it, a card taken having left. Narset, Parter of Veils, Once Upon a Time and United Battlefront print it.
pub fn rest_to_bottom_in_random_order(g: &mut dyn Game, player: PlayerId, looked: &[ObjectId]) {
    let rest: Vec<ObjectId> = looked.iter().copied().filter(|c| is_live(g, *c) && g.obj(*c).zone == Zone::library(player)).collect();
    g.bottom_in_random_order(player, &rest);
}

/// "Reveal the top `count` cards of your library. You may put a card that passes `wanted` from among them into your hand.
/// Put the rest into your graveyard" (CR 701.20, 400.3): the rest go together, in the order their owner arranges them (CR
/// 404.3, asked by the core). With nothing to take
/// nobody is asked. Satyr Wayfinder (a land card) and Malevolent Rumble (a permanent card) print it.
pub fn reveal_top_take_one_rest_to_graveyard(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    count: usize,
    wanted: fn(&Chars) -> bool,
) -> Result<(), Suspend> {
    top_take_one_rest_to_graveyard(g, player, source, count, true, 0, wanted)
}

/// "Look at the top `count` cards of your library. Put one of those cards into your hand and the rest into your graveyard"
/// (CR 701.20a, 400.3): the card is not revealed and taking it is not optional, so a library with any card shows the player
/// what it has and one is taken (CR 609.3 for a short library); the rest go together, in the order their owner arranges them
/// (CR 404.3). Stirring Honormancer prints it.
pub fn look_at_top_take_one_rest_to_graveyard(g: &mut dyn Game, player: PlayerId, source: ObjectId, count: usize) -> Result<(), Suspend> {
    top_take_one_rest_to_graveyard(g, player, source, count, false, 1, |_| true)
}

fn top_take_one_rest_to_graveyard(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    count: usize,
    reveal: bool,
    min: u8,
    wanted: fn(&Chars) -> bool,
) -> Result<(), Suspend> {
    let revealed = top_of_library(g, player, count);
    if revealed.is_empty() {
        return Ok(());
    }
    if reveal {
        g.reveal(player, &revealed);
    } else {
        g.look_at(player, &revealed);
    }
    let takeable: Vec<ObjectId> = revealed.iter().copied().filter(|&c| wanted(&g.chars(c))).collect();
    let mut taken = Vec::new();
    if !takeable.is_empty() {
        let ask = RequestKind::DistributeCards {
            source,
            cards: revealed.clone(),
            destinations: vec![
                Destination { kind: DestinationKind::Hand, min, max: 1, eligible: takeable },
                Destination { kind: DestinationKind::Graveyard, min: 0, max: u8::MAX, eligible: revealed.clone() },
            ],
            may_shuffle: false,
        };
        if let Answer::Placements { placements, .. } = g.ask(player, ask)? {
            taken = placements[0].clone();
        }
    }
    if reveal {
        g.return_to_hand_revealed(&taken);
    } else {
        g.return_to_hand(&taken);
    }
    let rest: Vec<ObjectId> = revealed.into_iter().filter(|c| !taken.contains(c)).collect();
    g.put_into_graveyard(&rest);
    Ok(())
}

/// "Mill `count` cards. You may put a [`kind`] card from among them into your hand" (CR 701.17, 701.17c): the cards milled
/// are the ones it may take from, as they are where they went (a graveyard, or exile by a replacement effect); `wanted`
/// says which of them qualify, `kind` is how the question words it. Nobody is asked when none qualifies, and choosing none
/// declines. The card put into the hand, as it is there (a replacement may send it elsewhere), is returned for a card that
/// asks what it was. Six and Town Greeter (a land), Tetzin, Gnome Champion (an artifact), Eerie Gravestone and Barrowgoyf
/// (a creature, the latter after its own "you may mill"), Dredger's Insight (an artifact, creature, or land) and Rick Jones, Destined
/// Sidekick (a Hero or enchantment, `wanted` capturing the Hero word of its text, CR 612) print it.
pub fn mill_then_may_take_into_hand(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    count: u32,
    kind: &'static str,
    wanted: impl Fn(&Chars) -> bool,
) -> Result<Vec<ObjectId>, Suspend> {
    let milled = g.mill(player, count);
    let takeable: Vec<ObjectId> = milled.iter().copied().filter(|&c| wanted(&g.chars(c))).collect();
    let chosen = crate::choose::cards_from_kind(g, player, source, Purpose::PutIntoHand, kind, milled, takeable, 1, true)?;
    Ok(g.return_to_hand(&chosen))
}

/// "Mill `count` cards. Then put a [`wanted`] card from among the milled cards onto the battlefield" (CR 701.17, 701.17c): the
/// card is not optional, one is chosen whenever any milled card qualifies, and the milled cards are found where they went (a
/// graveyard, or exile by a replacement effect). An Aura chooses what it enchants as it enters (CR 303.4f). Fetch Quest
/// (a creature, enchantment, or land) and Bind to Life (a creature) print it.
pub fn mill_then_put_onto_battlefield(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    count: u32,
    wanted: fn(&Chars) -> bool,
) -> Result<(), Suspend> {
    let among = g.mill(player, count);
    let eligible: Vec<ObjectId> = among.iter().copied().filter(|&c| wanted(&g.chars(c))).collect();
    if let Some(card) = crate::choose::card_from(g, player, source, Purpose::PutOntoBattlefield, among, eligible)? {
        g.put_onto_battlefield(card, player, false)?;
    }
    Ok(())
}

/// "Mill `count` cards. When you do, [`reflexive`]" (CR 701.17, 603.12): the reflexive ability triggers when the controller
/// milled that many cards, which they cannot if the library holds fewer (CR 701.17b: they mill what they can, and
/// nothing triggers) or a replacement effect kept a card from being milled. It chooses its target as it goes on the
/// stack, the milled cards included. Primal Witchstalker, Yathan Roadwatcher, Wick's Patrol, The Last Ronin (II) and
/// Kraven's Last Hunt (I) print it.
pub fn mill_then_reflexive(g: &mut dyn Game, item: &StackItem, count: u32, reflexive: AbilityRef) {
    if g.mill(item.controller, count).len() as u32 == count {
        g.reflexive_trigger(item.object, item.controller, reflexive, Target::None);
    }
}

/// "In any order" (CR 401.4): `player` puts `cards` on one `end` of their library (`DestinationKind::LibraryTop` or
/// `LibraryBottom`) in the order they choose, the first highest and the last lowest. Nobody is asked about fewer than two cards. The cards in
/// that order, for `put_into_library` or `arrange_library`. Bow of Nylea and Valakut Awakening put cards on the bottom,
/// Lim-Dul's Vault on either end.
pub fn ask_library_order(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    cards: &[ObjectId],
    end: DestinationKind,
) -> Result<Vec<ObjectId>, Suspend> {
    if cards.len() < 2 {
        return Ok(cards.to_vec());
    }
    let ask = RequestKind::DistributeCards {
        source,
        cards: cards.to_vec(),
        destinations: vec![Destination { kind: end, min: 0, max: u8::MAX, eligible: cards.to_vec() }],
        may_shuffle: false,
    };
    Ok(match g.ask(player, ask)? {
        Answer::Placements { placements, .. } if placements.first().is_some_and(|p| p.len() == cards.len()) => placements[0].clone(),
        _ => cards.to_vec(),
    })
}

/// "Reveal the top card of your library and put it into your hand" (CR 701.20, 400.3): a move, not a draw. The card, as
/// it is in the hand, and its mana value as it was revealed (CR 608.2h); nothing from an empty library. Dark Confidant,
/// Ad Nauseam and Caustic Bronco make the player lose life equal to that value.
pub fn reveal_top_to_hand(g: &mut dyn Game, player: PlayerId) -> Option<(ObjectId, u32)> {
    let &top = top_of_library(g, player, 1).first()?;
    let value = g.mana_value(top);
    g.reveal(player, &[top]);
    let moved = g.return_to_hand_revealed(&[top]);
    Some((moved.first().copied().unwrap_or(top), value))
}

/// Whether `card` is still the card it was and in exile (CR 400.7): "if it's still exiled", after it may have been cast or
/// played from there, or moved by a replacement effect. Beseech the Mirror, Etali, Primal Conqueror, Tersa Lightshatter and
/// Dragonhawk, Fate's Tempest ask it.
pub fn is_still_exiled(q: &dyn Query, card: ObjectId) -> bool {
    is_live(q, card) && q.obj(card).zone.is(ZoneKind::Exile)
}

/// The `cards` that are still exiled (`is_still_exiled`), in order. Crabomination, Dragonhawk, Fate's Tempest and cascade's
/// "put the rest on the bottom" ask it.
pub fn still_exiled(q: &dyn Query, cards: &[ObjectId]) -> Vec<ObjectId> {
    cards.iter().copied().filter(|&c| is_still_exiled(q, c)).collect()
}

/// "Exile the top `count` cards of `from`'s library. `who` may play them" under `grant` (CR 406, 305.1, 601.3): the cards
/// leave together, face up, and each that really reached exile (a replacement may send it elsewhere) can be played
/// under the grant, a land among them under the ordinary land-play rules. The cards still in exile. Laelia, the Blade
/// Reforged (one, this turn), Showdown of the Skalds (four, until the end of your next turn) and Mind's Desire print it.
pub fn exile_top_may_play(g: &mut dyn Game, from: PlayerId, count: usize, who: PlayerId, grant: PlayGrant) -> Vec<ObjectId> {
    let top = top_of_library(g, from, count);
    let mut exiled = Vec::new();
    for card in g.exile_all(&top) {
        if g.obj(card).zone.is(ZoneKind::Exile) {
            g.allow_play(card, who, grant);
            exiled.push(card);
        }
    }
    exiled
}

/// "Exile the top `count` cards of your library. Choose one of them. [Until ...] you may play that card" (CR 406, 305.1,
/// 601.3): the cards leave together, `player` chooses one of those that really reached exile (a replacement may send one
/// elsewhere), and only that card can be played, under `grant`; the others stay in exile unplayable. The card chosen, none
/// when nothing reached exile. Fireglass Mentor (until end of turn) and Heroes' Hangout (until the end of your next turn)
/// print it.
pub fn exile_top_choose_one_may_play(g: &mut dyn Game, player: PlayerId, source: ObjectId, count: usize, grant: PlayGrant) -> Result<Option<ObjectId>, Suspend> {
    let top = top_of_library(g, player, count);
    let exiled: Vec<ObjectId> = g.exile_all(&top).into_iter().filter(|&c| g.obj(c).zone.is(ZoneKind::Exile)).collect();
    let chosen = crate::choose::card(g, player, source, Purpose::Play, exiled)?;
    if let Some(card) = chosen {
        g.allow_play(card, player, grant);
    }
    Ok(chosen)
}

/// "Exile the top `count` cards of `from`'s library face down. `who` may look at and play [it / those cards]" under `grant`
/// (CR 406.3, 601.3, 305.1): the cards leave together face down, `who` looks at each that really reached exile (a
/// replacement may send it elsewhere), and may play it under the grant. The cards still in exile. Headliner Scarlett (one,
/// this turn) and Outrageous Robbery (X, for as long as they remain exiled) print it.
pub fn exile_top_face_down_may_look_and_play(g: &mut dyn Game, from: PlayerId, count: usize, who: PlayerId, grant: PlayGrant) -> Vec<ObjectId> {
    let top = top_of_library(g, from, count);
    let exiled: Vec<ObjectId> =
        g.exile_face_down(&top, None).into_iter().filter(|&card| g.obj(card).zone.is(ZoneKind::Exile)).collect();
    g.look_at(who, &exiled);
    for &card in &exiled {
        g.allow_play(card, who, grant);
    }
    exiled
}

/// "Exile the top `count` cards of your library. Until the end of your next turn, you may play [it / those cards]"
/// (`exile_top_may_play`): the grant is to play, not only cast, a land among them under the ordinary land-play rules, until the
/// cleanup step of `player`'s next turn (CR 514.2). The cards still in exile. Cori Mountain Monastery (one), Clockwork
/// Percussionist (one), The Legend of Roku (three) and Showdown of the Skalds (four) print it.
pub fn exile_top_play_until_end_of_next_turn(g: &mut dyn Game, player: PlayerId, count: usize) -> Vec<ObjectId> {
    let grant = PlayGrant { instead: None, lands: true, expiry: Expiry::EndOfYourNextTurn, ..PlayGrant::CAST };
    exile_top_may_play(g, player, count, player, grant)
}

/// "Exile the top card of your library. You may play it until the end of your next turn" as a resolving ability's whole
/// effect: `exile_top_play_until_end_of_next_turn` for the ability's controller, one card. An empty library exiles nothing.
/// Clockwork Percussionist, Gundabad Opportunist and Snowslope Hunter print it.
pub fn exile_top_card_play_until_end_of_next_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_top_play_until_end_of_next_turn(g, item.controller, 1);
    Ok(())
}

/// "Exile the top `count` cards of your library. You may play [it / those cards] this turn" (`exile_top_may_play`): the grant
/// is to play, not only cast, a land among them under the ordinary land-play rules, until the cleanup step (CR 514.2). The
/// cards still in exile. Laelia, the Blade Reforged, Emberheart Challenger and Experimental Synthesizer (one card each) print it.
pub fn exile_top_play_this_turn(g: &mut dyn Game, player: PlayerId, count: usize) -> Vec<ObjectId> {
    let grant = PlayGrant { lands: true, expiry: Expiry::EndOfTurn, ..PlayGrant::CAST };
    exile_top_may_play(g, player, count, player, grant)
}

/// "Mill a card. You may play that card this turn" as a resolving ability's whole effect (CR 701.17, 601.3, 305.1): the card the
/// mill put into the graveyard is the object that may be played, a land under the ordinary land-play rules, until the cleanup
/// step (CR 514.2); with an empty library nothing is milled and nothing is granted. Tablet of Discovery and Ark of Hunger print it.
pub fn mill_a_card_and_play_it_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for card in g.mill(item.controller, 1) {
        g.allow_play(card, item.controller, PlayGrant { lands: true, expiry: Expiry::EndOfTurn, ..PlayGrant::CAST });
    }
    Ok(())
}

/// "Exile the top `count` cards of your library. You may play [it / those cards] until your next end step"
/// (`exile_top_may_play`): the grant is to play, not only cast, so a land among them may be the turn's land drop (CR 305.1),
/// and it ends as `player`'s next end step begins (CR 500.4). The cards still in exile. Dragonhawk, Fate's Tempest (X cards),
/// Inti, Seneschal of the Sun (one) and Questing Druid's Seek the Beast (two) print it.
pub fn exile_top_play_until_your_next_end_step(g: &mut dyn Game, player: PlayerId, count: usize) -> Vec<ObjectId> {
    let grant = PlayGrant { lands: true, expiry: Expiry::UntilYourNextEndStep, ..PlayGrant::CAST };
    exile_top_may_play(g, player, count, player, grant)
}

/// "Exile the top card of your library. You may play that card for as long as you control [`source`]"
/// (`exile_top_may_play`): the grant is to play, not only cast, so a land may be the turn's land drop (CR 305.1), and it
/// lasts while `player` controls `source` on the battlefield, ending for good the first time they don't - it leaves,
/// phases out or changes controller - and does nothing if that is already so as it resolves (CR 611.2b,
/// `Expiry::WhileControls`). The card still in exile. Gwen Stacy and Lightning, Security Sergeant print it.
pub fn exile_top_play_while_you_control(g: &mut dyn Game, player: PlayerId, source: ObjectId) -> Vec<ObjectId> {
    let grant = PlayGrant { lands: true, expiry: Expiry::WhileControls(source), ..PlayGrant::CAST };
    exile_top_may_play(g, player, 1, player, grant)
}

/// "[One] of `items` chosen at random": one is drawn from the game's own generator, so the same game picks
/// the same one; no draw is made when there is no choice. None from an empty list. Crabomination picks a card at random
/// from a graveyard and a hand, Chaos Defiler one of the creatures chosen, Ursine Monstrosity an opponent and Tersa
/// Lightshatter a card of her graveyard.
pub fn random_of<T: Copy>(g: &mut dyn Game, items: &[T]) -> Option<T> {
    match items.len() {
        0 => None,
        1 => items.first().copied(),
        n => items.get(g.choose_at_random(n)).copied(),
    }
}

/// Whether `card` has the name `wanted`, the name of a card (CR 201.2a). A split card has the name of each half in
/// common with a card of that name (CR 201.2a, 201.4b: a player naming a split card names one of its halves).
pub fn card_has_name(q: &dyn Query, card: CardId, wanted: CardId) -> bool {
    let name = q.card(wanted).name;
    let def = q.card(card);
    def.name == name || def.split.is_some_and(|(left, right)| left.name == name || right.name == name)
}

/// The names the object `id` has (CR 201.2a): a Room on the battlefield those of its unlocked doors alone, and none at all
/// with neither unlocked (CR 709.5); any other split card both halves' (CR 709.4a); a copy that says "except its name is X"
/// only that (CR 707.9b), and so has one whose name an effect set (CR 612.8); and anything else its own name, one for each
/// of the two faces' names of a card written "A // B".
pub fn object_names(q: &dyn Query, id: ObjectId) -> Vec<&'static str> {
    let c = q.chars(id);
    let obj = q.obj(id);
    if c.name_set || (obj.face_down_values().is_none() && obj.copy.as_ref().is_some_and(|copy| copy.name.is_some())) {
        return vec![c.name];
    }
    let def = q.card(c.card);
    if def.is_room() && obj.zone.is(ZoneKind::Battlefield) {
        return def.unlocked_doors(obj.marks.doors).map(|half| half.name).collect();
    }
    // One that has left is as it last was (CR 608.2h): a Room the names of the doors it had unlocked, "A // B" both.
    if def.is_room() && q.state().last_known(id).is_some() {
        return c.name.split(" // ").filter(|name| !name.is_empty()).collect();
    }
    if let Some((left, right)) = def.split {
        return vec![left.name, right.name];
    }
    c.name.split(" // ").filter(|name| !name.is_empty()).collect()
}

/// Whether the object `id` has the name `wanted` (CR 201.2a, 709.4a): "an object has the chosen name if one of its names is
/// the chosen name" (`object_names`). Anointed Peacekeeper's "sources with the chosen name", Tamiyo, Collector of Tales and any
/// card that asks of a permanent, a spell or a card in a zone whether it is named so.
pub fn object_has_name(q: &dyn Query, id: ObjectId, wanted: CardId) -> bool {
    let name = q.card(wanted).name;
    object_names(q, id).contains(&name)
}

/// "Reveal cards from the top of your library until you reveal a card that `qualifies`" (CR 701.20, 400.3):
/// `reveal_until_n` for one card. The card that qualified, if any, and the rest of the revealed cards in the order
/// revealed, for the caller to put "on the bottom of your library in a random order" (`Game::bottom_in_random_order`).
/// Raph & Mikey (a creature card), Heirloom Blade (a creature card sharing a type with the creature), Identity Echo (a
/// creature or planeswalker card), Oath of Druids (a creature card) and Indomitable Creativity (an artifact or creature
/// card) print it.
pub fn reveal_until(g: &mut dyn Game, player: PlayerId, qualifies: &dyn Fn(&dyn Query, ObjectId) -> bool) -> (Option<ObjectId>, Vec<ObjectId>) {
    let (found, rest) = reveal_until_n(g, player, 1, qualifies);
    (found.first().copied(), rest)
}

/// "Reveal cards from the top of your library until you reveal `n` cards that `qualifies`" (CR 701.20, 400.3): the cards
/// are revealed together from the top down to the `n`th that qualifies, or the whole library when it holds fewer (nothing
/// from an empty one, and nothing at all for `n` of 0). The cards that qualified, in the order revealed, and every other
/// revealed card, in the order revealed, for the caller to put where they go. Dack Fayden, Helping Hand (X creature cards,
/// X the number of opponents you have).
pub fn reveal_until_n(
    g: &mut dyn Game,
    player: PlayerId,
    n: usize,
    qualifies: &dyn Fn(&dyn Query, ObjectId) -> bool,
) -> (Vec<ObjectId>, Vec<ObjectId>) {
    let mut found: Vec<ObjectId> = Vec::new();
    let mut rest: Vec<ObjectId> = Vec::new();
    let mut shown: Vec<ObjectId> = Vec::new();
    for card in g.state().zone(Zone::library(player)).to_vec() {
        if found.len() == n {
            break;
        }
        shown.push(card);
        if qualifies(&*g, card) {
            found.push(card);
        } else {
            rest.push(card);
        }
    }
    if !shown.is_empty() {
        g.reveal(player, &shown);
    }
    (found, rest)
}

/// "Exile cards from the top of your library until you exile a nonland card [that `qualifies`]" (CR 701.57a, 702.85a): each
/// card is exiled one at a time, face up, from the top until one is a nonland card `qualifies` accepts, or the library
/// is empty. Every card that really reached exile, in order (the qualifying one last), and the qualifying card if there
/// was one; a card a replacement effect sent elsewhere is neither. Cascade (a lesser mana value than the spell's) and
/// discover (a mana value of N or less) use it.
pub fn exile_until_nonland(
    g: &mut dyn Game,
    player: PlayerId,
    qualifies: &dyn Fn(&dyn Query, ObjectId) -> bool,
) -> (Vec<ObjectId>, Option<ObjectId>) {
    let mut exiled = Vec::new();
    while let Some(&top) = g.state().zone(Zone::library(player)).first() {
        let moved = g.exile(top);
        if !g.obj(moved).zone.is(ZoneKind::Exile) {
            continue;
        }
        exiled.push(moved);
        if !g.chars(moved).is(CardType::Land) && qualifies(&*g, moved) {
            return (exiled, Some(moved));
        }
    }
    (exiled, None)
}

/// "An instant or sorcery card" as a search finds it (CR 205.2a, 701.23): the card's types as they are in the library, and as a
/// card milled into a graveyard has them, or a spell on the stack. Mystical Tutor and Wild Idea (Sanar, Unfinished Genius's
/// prepare spell) search for it; Gleam of Death takes them from among the cards it milled and Glamdring, Foe-hammer reduces
/// the cost of the spells that are them.
pub fn instant_or_sorcery_card(q: &dyn Query, card: ObjectId) -> bool {
    let chars = q.chars(card);
    chars.is(CardType::Instant) || chars.is(CardType::Sorcery)
}

/// "[Player] shuffles their graveyard into their library" (CR 701.24c): every card in the graveyard goes into the library
/// at once and the library is shuffled, which it is even when no card arrives. Emrakul, the Aeons Torn, Spirit Water
/// Revival and Quest for Ancient Secrets print it.
pub fn shuffle_graveyard_into_library(g: &mut dyn Game, player: PlayerId) {
    let graveyard: Vec<ObjectId> = g.state().zone(Zone::graveyard(player)).to_vec();
    g.put_into_library(&graveyard, LibraryPlace::Top);
    g.shuffle(player);
}

/// "Put target card from a graveyard on the bottom of its owner's library" (CR 401.4): the first target goes there as the
/// ability resolves, if it is still the card that was targeted (CR 608.2b); the library it joins is its owner's, whatever
/// graveyard it was in. Paradox Shaper (a card from your graveyard) and Sundering Archaic (from a graveyard) print it.
pub fn put_first_target_on_the_bottom(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(card) = crate::targets::object(g, item, 0) {
        g.put_into_library(&[card], LibraryPlace::Bottom);
    }
    Ok(())
}

const TOP_OR_BOTTOM: Question = Question::new("PutOnTop", "put it on top of its owner's library?", "If you don't, it goes on the bottom", "Top", "Bottom");

/// "Its owner puts it on their choice of the top or bottom of their library": the owner of `object` (CR 108.3), not its
/// controller, chooses, and yes is the top. A spell on the stack leaves it without being countered (CR 701.6a does not
/// apply); anything else is put into the library (CR 401.1). Subtlety, Horned Loch-Whale, Plan for All Outcomes and Aether
/// Gust print it.
pub fn owner_puts_on_top_or_bottom(g: &mut dyn Game, source: ObjectId, object: ObjectId) -> Result<(), Suspend> {
    let owner = g.obj(object).owner;
    let top = crate::effects::ask_may(g, owner, source, Purpose::Ask(&TOP_OR_BOTTOM))?;
    if g.state().stack.iter().any(|i| i.kind == StackKind::Spell && i.object == object) {
        g.remove_spell(object, if top { CounterTo::LibraryTop } else { CounterTo::LibraryBottom });
    } else {
        g.put_into_library(&[object], if top { LibraryPlace::Top } else { LibraryPlace::Bottom });
    }
    Ok(())
}

/// "Target creature's owner puts it on their choice of the top or bottom of their library": the first target's owner
/// chooses (`owner_puts_on_top_or_bottom`). With its target gone the spell does not resolve (CR 608.2b). Uneasy Partings, Lagoon
/// Breach (Horned Loch-Whale) and Run Behind print it.
pub fn owner_puts_first_target_on_top_or_bottom(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = crate::targets::object(g, item, 0) else { return Ok(()) };
    owner_puts_on_top_or_bottom(g, item.object, creature)
}
