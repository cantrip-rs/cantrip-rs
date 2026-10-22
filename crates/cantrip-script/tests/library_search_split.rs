//! "Search your library for a creature or land card and reveal it. Put it onto the battlefield tapped if it's a land card.
//! Otherwise, put it into your hand. Then shuffle." (`cantrip_mechanics::library::search_land_tapped_else_hand`, CR 701.23,
//! 305.4), on a table of test-only cards: the search tells the searcher where each kind of card goes, the card found is
//! revealed, a land enters tapped, anything else goes to the hand, a land creature is a land card, and finding nothing
//! moves nothing. The printed case is Archdruid's Charm's first mode.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_mechanics::library::{Find, search_land_tapped_else_hand};

/// "{0}: Search your library for a creature or land card and reveal it. Put it onto the battlefield tapped if it's a land
/// card. Otherwise, put it into your hand. Then shuffle."
fn search(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    search_land_tapped_else_hand(g, item.controller, item.object, Find::ONE.revealed(), |q, c| {
        let chars = q.chars(c);
        chars.is(CardType::Creature) || chars.is(CardType::Land)
    })?;
    Ok(())
}

static SEEKER: CardDef = CardDef {
    name: "Probe Seeker",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(
        cost("{0}"),
        search,
        "{0}: Search your library for a creature or land card and reveal it. Put it onto the battlefield tapped if it's a land card. Otherwise, put it into your hand. Then shuffle.",
    )],
    ..CardDef::BLANK
};

static BEAR: CardDef = CardDef {
    name: "Probe Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// A land creature (Dryad Arbor): a land card, so it enters tapped rather than going to the hand.
static ARBOR: CardDef = CardDef {
    name: "Probe Arbor",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land, CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

static RITUAL: CardDef = CardDef { name: "Probe Ritual", types: TypeSet::of(&[CardType::Sorcery]), ..CardDef::BLANK };

static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&SEEKER, &BEAR, &ARBOR, &RITUAL, &FILLER]))
}

/// Bot1 casts Probe Seeker and activates it on turn 1, answering the search with the card named `pick` (or none). The
/// game, the card found, and the search question's destination and eligible cards' names.
fn play(pick: Option<&str>) -> (Game, Option<ObjectId>, SearchDestination, Vec<ObjectName>) {
    let filler = table().find("Filler").unwrap();
    let find = |n: &str| table().find(n).unwrap();
    // The opening hand is the first seven; the library below it holds one of each kind, then fillers.
    let mut deck: Vec<CardId> = vec![find("Probe Seeker")];
    deck.extend(std::iter::repeat(filler).take(6));
    deck.extend([find("Probe Ritual"), find("Probe Bear"), find("Probe Arbor")]);
    deck.extend(std::iter::repeat(filler).take(30 - deck.len()));
    let mut game = Game::new(table(), &Setup { decks: vec![deck, vec![filler; 30]], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut steps = [(ActionKind::CastSpell, 0u8), (ActionKind::ActivateAbility, 0u8)].into_iter().peekable();
    let mut asked = None;
    for _ in 0..200 {
        game.run();
        if steps.peek().is_none() && asked.is_some() && game.state.stack.is_empty() && game.state.triggers.is_empty() {
            break;
        }
        let Some(request) = game.pending().cloned() else { panic!("the game ended") };
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) => {
                let option = steps.peek().and_then(|&(kind, index)| {
                    options.iter().find(|o| o.kind == kind && o.index == index && o.source.is_some_and(|s| game.name(s) == "Probe Seeker"))
                });
                match option {
                    Some(option) => {
                        steps.next();
                        Answer::Act(Action::of(option))
                    }
                    None => Answer::Pass,
                }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::SearchLibrary { destination, .. } => {
                let eligible = first_choices(&game, &request);
                let names: Vec<ObjectName> = eligible.iter().map(|&c| game.name(c)).collect();
                let chosen: Vec<ObjectId> = pick.map(|p| eligible.iter().copied().filter(|&c| game.name(c) == p).take(1).collect()).unwrap_or_default();
                asked = Some((*destination, names, chosen.first().copied()));
                Answer::Cards(chosen)
            }
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    let (destination, names, found) = asked.expect("the search was asked");
    (game, found, destination, names)
}

/// The card named `name` that `player` controls on the battlefield, and whether it is tapped.
fn on_battlefield(game: &Game, name: &str) -> Option<bool> {
    game.state.battlefield().iter().find(|&&c| game.name(c) == name).map(|&c| game.state.obj(c).tapped)
}

fn in_hand(game: &Game, name: &str) -> bool {
    game.state.zone(Zone::hand(PlayerId(0))).iter().any(|&c| game.name(c) == name)
}

fn revealed(game: &Game, card: ObjectId) -> bool {
    game.history().any(EventQuery::kind(EventMask::REVEALED).object(card))
}

/// The question says where each kind of card goes, and offers creature and land cards but not the sorcery (CR 701.23a).
#[test]
fn the_search_announces_the_split_and_offers_only_creature_and_land_cards() {
    let (_, _, destination, names) = play(None);
    assert_eq!(destination, SearchDestination::LandTappedElseHand);
    assert!(names.contains(&ObjectName::Literal("Probe Bear")) && names.contains(&ObjectName::Literal("Probe Arbor")) && names.contains(&ObjectName::Literal("Filler")));
    assert!(!names.contains(&ObjectName::Literal("Probe Ritual")));
}

/// A land card found is revealed and put onto the battlefield tapped, not into the hand (CR 305.4: put, not played).
#[test]
fn a_land_card_is_revealed_and_enters_tapped() {
    let (game, found, _, _) = play(Some("Filler"));
    assert!(revealed(&game, found.unwrap()));
    let fillers_on_battlefield: Vec<bool> =
        game.state.battlefield().iter().filter(|&&c| game.name(c) == "Filler").map(|&c| game.state.obj(c).tapped).collect();
    assert_eq!(fillers_on_battlefield, vec![true]);
    assert_eq!(game.state.zone(Zone::hand(PlayerId(0))).len(), 6);
}

/// A creature card found is revealed and put into the hand.
#[test]
fn a_creature_card_is_revealed_and_goes_to_the_hand() {
    let (game, found, _, _) = play(Some("Probe Bear"));
    assert!(revealed(&game, found.unwrap()));
    assert!(in_hand(&game, "Probe Bear"));
    assert_eq!(on_battlefield(&game, "Probe Bear"), None);
    assert_eq!(game.state.zone(Zone::hand(PlayerId(0))).len(), 7);
}

/// A land creature card is a land card: onto the battlefield tapped.
#[test]
fn a_land_creature_card_is_a_land_card() {
    let (game, _, _, _) = play(Some("Probe Arbor"));
    assert_eq!(on_battlefield(&game, "Probe Arbor"), Some(true));
    assert!(!in_hand(&game, "Probe Arbor"));
}

/// Finding nothing (CR 701.23b) moves nothing and reveals nothing.
#[test]
fn finding_nothing_moves_nothing() {
    let (game, found, _, _) = play(None);
    assert_eq!(found, None);
    assert_eq!(game.state.zone(Zone::library(PlayerId(0))).len(), 23);
    assert_eq!(game.state.zone(Zone::hand(PlayerId(0))).len(), 6);
    assert!(!game.history().any(EventQuery::kind(EventMask::REVEALED)));
}
