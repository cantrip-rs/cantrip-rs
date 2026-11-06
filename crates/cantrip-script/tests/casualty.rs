//! Casualty (CR 702.153a) and a copy of a spell made with exceptions (CR 707.9b), played on a table of test-only cards
//! until the printed ones (Ob Nixilis, the Adversary; Illicit Shipment; Rob the Archives) have scenarios of their own:
//!
//! - casualty X announces X once the intention to pay is announced (CR 601.2b, 107.3a), and only a creature with power X
//!   or greater as the cost is paid pays it (CR 601.2h) - its power read after the mana abilities activated while paying
//!   (CR 601.2g), which may raise it or lower it;
//! - the trigger copies the spell only when the cost was paid (CR 603.4), and the copy has the X the original was cast
//!   with (CR 707.10);
//! - casualty N offers the cost only with a creature of power N or greater to sacrifice;
//! - "the copy isn't legendary and has starting loyalty X": the copy of a permanent spell becomes a token as it resolves
//!   (CR 707.10f) with the copiable values the exceptions gave it, enters with that loyalty (CR 306.5b), and a copy of that
//!   token copies them (CR 707.9b).

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_mechanics::casualty;

/// "You gain X life", X as announced.
fn gain_x(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, item.x as i32);
    Ok(())
}

const fn body(name: &'static str, power: i32) -> CardDef {
    CardDef { name, types: TypeSet::of(&[CardType::Creature]), power: Some(power), toughness: Some(4), ..CardDef::BLANK }
}

static SMALL: CardDef = body("Small Body", 1);
static BIG: CardDef = body("Big Body", 3);
static NEGATIVE: CardDef = body("Negative Body", -1);

/// "This creature's power is equal to the number of cards in all graveyards."
fn power_is_graveyard_cards(q: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    let state = q.state();
    let cards: usize = state.seats().map(|p| state.zone(Zone::graveyard(p)).len()).sum();
    chars.power = Some(cards as i32);
}

/// "This creature's power is equal to the number of creatures you control."
fn power_is_creatures_you_control(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let state = q.state();
    let you = state.obj(source).controller;
    let creatures = state.battlefield().iter().filter(|&&id| state.obj(id).controller == you && q.chars(id).is(CardType::Creature)).count();
    chars.power = Some(creatures as i32);
}

/// A */4 that grows with the graveyards, as a Lhurgoyf does.
static GROWER: CardDef = CardDef {
    name: "Grower",
    statics: &[characteristic_defining(Layer::PtDefine, power_is_graveyard_cards, "This creature's power is equal to the number of cards in all graveyards.").lost_pt(0, 4)],
    ..body("Grower", 0)
};

/// A */4 that shrinks as your creatures go.
static CROWD: CardDef = CardDef {
    name: "Crowd",
    statics: &[characteristic_defining(Layer::PtDefine, power_is_creatures_you_control, "This creature's power is equal to the number of creatures you control.").lost_pt(0, 4)],
    ..body("Crowd", 0)
};

fn creature_you_control(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && q.controller(id) == asker.controller
}

/// "Sacrifice a creature: Add {C}" (Ashnod's Altar's shape): a mana ability activated while paying (CR 601.2g).
static ALTAR: CardDef = CardDef {
    name: "Probe Altar",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(Cost::sacrifice(1, creature_you_control), add(Mana::C), "Sacrifice a creature: Add {C}.")],
    ..CardDef::BLANK
};

const LABELS: &[&str] = &["Cast without casualty", "Casualty: sacrifice a creature"];

/// A zero-cost sorcery with casualty X: "You gain X life."
static X_PROBE: CardDef = CardDef {
    name: "Casualty X Probe",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], gain_x),
    optional_cost: Some(casualty::X),
    labels: LABELS,
    abilities: &[casualty::trigger("Casualty X")],
    ..CardDef::BLANK
};

/// The same for {1}, paid with the altar's mana.
static PAID_X_PROBE: CardDef = CardDef { name: "Casualty X Probe For One", cost: ManaCost { generic: 1, ..ManaCost::ZERO }, ..X_PROBE };

/// A zero-cost sorcery with casualty 2 that does nothing but gain X (0) life.
static TWO_PROBE: CardDef = CardDef { name: "Casualty Two Probe", optional_cost: Some(casualty::optional::<2>()), abilities: &[casualty::trigger("Casualty 2")], ..X_PROBE };

/// "The copy isn't legendary and has starting loyalty X."
fn copy_nonlegendary(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let loyalty = casualty::x(g, item) as i32;
    let values = g.copiable_values(item.object).nonlegendary().with_loyalty(loyalty);
    g.copy_spell_as(item.object, item.controller, true, values)?;
    Ok(())
}

/// A zero-cost legendary planeswalker with loyalty 3 and casualty X, whose copy isn't legendary and has starting loyalty X.
static WALKER: CardDef = CardDef {
    name: "Casualty Walker",
    types: TypeSet::of(&[CardType::Planeswalker]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false },
    loyalty: Some(3),
    optional_cost: Some(casualty::X),
    labels: LABELS,
    abilities: &[casualty::trigger_with("Casualty X. The copy isn't legendary and has starting loyalty X.", copy_nonlegendary)],
    ..CardDef::BLANK
};

/// "Create a token that's a copy of a planeswalker token you control."
fn copy_a_walker_token(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let token = g.state().battlefield().iter().copied().find(|&id| g.obj(id).token && g.chars(id).is(CardType::Planeswalker));
    if let Some(token) = token {
        g.create_token_copies(token, item.controller, 1)?;
    }
    Ok(())
}

static MIRROR: CardDef = CardDef { name: "Walker Mirror", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], copy_a_walker_token), ..CardDef::BLANK };

static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        CardTable::new([&SMALL, &BIG, &NEGATIVE, &GROWER, &CROWD, &ALTAR, &X_PROBE, &PAID_X_PROBE, &TWO_PROBE, &WALKER, &MIRROR, &FILLER])
    })
}

/// A two-player game: the first seat's hand is the named cards, then lands; `board` is on its battlefield.
fn deal(hand: &[&str], board: &[&str]) -> Game {
    deal_with_graveyard(hand, board, &[])
}

/// `deal`, with `graveyard` in the first seat's graveyard.
fn deal_with_graveyard(hand: &[&str], board: &[&str], graveyard: &[&str]) -> Game {
    let list = |names: &[&str]| -> Vec<CardId> {
        let mut deck: Vec<CardId> = names.iter().map(|n| table().find(n).unwrap_or_else(|| panic!("no card {n}"))).collect();
        let filler = table().find("Filler").unwrap();
        deck.extend(std::iter::repeat_n(filler, 30 - deck.len()));
        deck
    };
    let mut game = Game::new(table(), &Setup { decks: vec![list(hand), list(&[])], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    for name in board {
        let id = game.state.push_obj(Obj::new(table().find(name).unwrap(), PlayerId(0), Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
    }
    for name in graveyard {
        let id = game.state.push_obj(Obj::new(table().find(name).unwrap(), PlayerId(0), Zone::graveyard(PlayerId(0))));
        game.state.zone_push(Zone::graveyard(PlayerId(0)), id);
    }
    game.run();
    game
}

/// What the casting asked, as `play` saw it.
#[derive(Default, Debug)]
struct Seen {
    /// Whether paying the casualty cost was on offer, at each announcement of it.
    offered: Vec<bool>,
    /// The most X could be announced as, at each announcement of X.
    most: Vec<u32>,
    /// The names of the creatures offered to pay the casualty sacrifice, at each choice of them.
    sacrificable: Vec<Vec<&'static str>>,
}

/// How `play` casts: whether it pays casualty when it is on offer, the X it announces, the creature the casualty cost
/// sacrifices, and the creature the altar sacrifices for mana while paying, if any.
#[derive(Clone, Copy)]
struct Plan<'a> {
    pay: bool,
    x: u32,
    victim: &'a str,
    altar: Option<&'a str>,
}

const fn paying<'a>(x: u32, victim: &'a str) -> Plan<'a> {
    Plan { pay: true, x, victim, altar: None }
}

/// Plays on: the first seat casts the named cards in order whenever it can, answering as `plan` says, and passes
/// otherwise, until the cards are cast and the stack is empty.
fn play(game: &mut Game, casts: &[&str], plan: Plan) -> Seen {
    let mut casts: Vec<&str> = casts.to_vec();
    let mut altar = plan.altar;
    let mut seen = Seen::default();
    for _ in 0..200 {
        game.run();
        let Some(request) = game.pending().cloned() else { panic!("the game ended") };
        // A cast part-way through its announcements is on no stack yet: done only at priority.
        if casts.is_empty() && game.state.stack.is_empty() && matches!(request.kind, RequestKind::Priority { .. }) {
            return seen;
        }
        let answer = match &request.kind {
            RequestKind::Priority { options } => {
                let option = casts.first().and_then(|&name| options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == name)));
                match option {
                    Some(option) if request.player == PlayerId(0) => {
                        casts.remove(0);
                        Answer::Act(Action::of(option))
                    }
                    _ => Answer::Pass,
                }
            }
            RequestKind::ChooseModes { eligible, additional_cost: true, .. } => {
                seen.offered.push(eligible.contains(&1));
                Answer::Modes(vec![if plan.pay && eligible.contains(&1) { 1 } else { 0 }])
            }
            RequestKind::PayCosts { announcing: true, most, .. } => {
                seen.most.push(*most);
                Answer::Announce(plan.x)
            }
            RequestKind::PayCosts { options, .. } => {
                let sacrifice = altar.and_then(|name| {
                    options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s) == "Probe Altar")).map(|o| {
                        let mut action = Action::of(o);
                        let victim = o.target_candidates.iter().copied().find(|t| matches!(t, Target::Object(id) if game.name(*id) == name));
                        action.targets = vec![victim.unwrap_or_else(|| panic!("the altar may not sacrifice {name}"))];
                        action
                    })
                });
                match sacrifice {
                    Some(action) => {
                        altar = None;
                        Answer::Act(action)
                    }
                    None => Answer::Act(Action::of(
                        options
                            .iter()
                            .find(|o| o.kind == ActionKind::ConfirmCasting)
                            .or_else(|| options.iter().find(|o| o.kind == ActionKind::AssignMana))
                            .unwrap_or_else(|| panic!("nothing to pay with: {options:?}")),
                    )),
                }
            }
            RequestKind::ChooseCostPayment { choices, .. } => {
                let choice = choices.iter().position(|c| c.label.starts_with("Sacrifice"))
                    .or_else(|| choices.iter().position(|c| c.label.starts_with("Pay mana")))
                    .expect("a casualty or mana payment is offered");
                Answer::CostPayment(choice)
            }
            RequestKind::ChooseCards { cards, purpose: Purpose::SacrificeCost, .. } => {
                seen.sacrificable.push(cards.iter().map(|&c| game.name(c)).collect());
                let pick = cards.iter().copied().find(|&c| game.name(c) == plan.victim).unwrap_or_else(|| panic!("{} may not be sacrificed", plan.victim));
                Answer::Cards(vec![pick])
            }
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the casts did not resolve: {seen:?}");
}

fn on_battlefield(game: &Game, name: &str) -> Vec<ObjectId> {
    game.state.battlefield().iter().copied().filter(|&id| game.name(id) == name).collect()
}

fn life(game: &Game) -> i32 {
    game.state.player(PlayerId(0)).life
}

fn copies(game: &Game) -> usize {
    game.log.iter().filter(|r| matches!(r.event, Event::Copied { .. })).count()
}

fn casts_of(game: &Game, name: &str) -> usize {
    game.log.iter().filter(|r| matches!(r.event, Event::SpellCast { spell, .. } if game.name(spell) == name)).count()
}

/// The most a power can be: X is announced up to it, since the power is read only as the cost is paid (CR 601.2h).
const MOST_POWER: u32 = i32::MAX as u32;

#[test]
fn casualty_x_sacrifices_a_creature_with_power_x_or_greater() {
    let mut game = deal(&["Casualty X Probe"], &["Small Body", "Big Body", "Negative Body"]);
    let before = life(&game);
    let seen = play(&mut game, &["Casualty X Probe"], paying(2, "Big Body"));
    assert_eq!(seen.offered, vec![true]);
    assert_eq!(seen.most, vec![MOST_POWER], "X goes up to the most a power can be");
    assert_eq!(seen.sacrificable, vec![vec!["Big Body"]], "only a creature with power 2 or greater pays");
    assert!(on_battlefield(&game, "Big Body").is_empty() && on_battlefield(&game, "Small Body").len() == 1);
    // The spell and its copy, which has the X it was cast with (CR 707.10), each gain 2.
    assert_eq!(life(&game), before + 4);
}

#[test]
fn casualty_unpaid_announces_no_x_and_copies_nothing() {
    let mut game = deal(&["Casualty X Probe"], &["Big Body"]);
    let before = life(&game);
    let seen = play(&mut game, &["Casualty X Probe"], Plan { pay: false, ..paying(2, "Big Body") });
    assert_eq!(seen.offered, vec![true]);
    assert!(seen.most.is_empty() && seen.sacrificable.is_empty());
    assert_eq!(on_battlefield(&game, "Big Body").len(), 1);
    assert_eq!(life(&game), before);
    assert_eq!(copies(&game), 0, "an unpaid casualty does not trigger");
}

/// A creature's power is read as the cost is paid (CR 601.2h), after the mana abilities (CR 601.2g): with nothing of power
/// 0 or more when the cost is announced, it is still on offer, and is taken back when nothing has the power at payment.
#[test]
fn casualty_x_with_only_negative_power_is_offered_and_taken_back_unpaid() {
    let mut game = deal(&["Casualty X Probe"], &["Negative Body"]);
    let seen = play(&mut game, &["Casualty X Probe"], paying(0, "Negative Body"));
    assert_eq!(seen.offered, vec![true], "a power read at payment may yet be 0 or more");
    assert!(seen.sacrificable.is_empty(), "nothing has power 0 or more at payment");
    assert_eq!(casts_of(&game, "Casualty X Probe"), 0, "the cast is taken back");
    assert_eq!(on_battlefield(&game, "Negative Body").len(), 1);
}

/// Lhurgoyf's case: X 2 announced while the greatest power is 1 (the Grower, with one card in a graveyard, and a 1/4); the
/// altar sacrifices the 1/4 for the mana while paying (CR 601.2g), a second card is in a graveyard, the Grower is 2/4 as the
/// cost is paid (CR 601.2h), and pays it.
#[test]
fn a_power_raised_by_a_mana_ability_while_paying_pays_the_larger_x() {
    let mut game = deal_with_graveyard(&["Casualty X Probe For One"], &["Grower", "Small Body", "Probe Altar"], &["Filler"]);
    let before = life(&game);
    let seen = play(&mut game, &["Casualty X Probe For One"], Plan { altar: Some("Small Body"), ..paying(2, "Grower") });
    assert_eq!(seen.sacrificable, vec![vec!["Grower"]], "the Grower is 2/4 by the time the cost is paid");
    assert!(on_battlefield(&game, "Grower").is_empty() && on_battlefield(&game, "Small Body").is_empty());
    assert_eq!(life(&game), before + 4, "the spell and its copy each gain 2");
}

/// The other way: X 2 announced over a creature of power 2 (it and another creature); the altar sacrifices the other while
/// paying, so it is 1/4 as the cost is paid and can't pay: the cast is taken back (CR 601.2h).
#[test]
fn a_power_lowered_below_x_while_paying_takes_the_cast_back() {
    let mut game = deal(&["Casualty X Probe For One"], &["Crowd", "Small Body", "Probe Altar"]);
    let seen = play(&mut game, &["Casualty X Probe For One"], Plan { altar: Some("Small Body"), ..paying(2, "Crowd") });
    assert_eq!(seen.most, vec![MOST_POWER]);
    assert!(seen.sacrificable.is_empty(), "nothing has power 2 at payment");
    assert_eq!(casts_of(&game, "Casualty X Probe For One"), 0, "the cast is taken back");
    assert_eq!(on_battlefield(&game, "Crowd").len(), 1);
}

#[test]
fn casualty_n_needs_a_creature_with_power_n_or_greater() {
    let mut game = deal(&["Casualty Two Probe"], &["Small Body"]);
    let seen = play(&mut game, &["Casualty Two Probe"], paying(0, "Small Body"));
    assert_eq!(seen.offered, vec![false]);

    let mut game = deal(&["Casualty Two Probe"], &["Small Body", "Big Body"]);
    let seen = play(&mut game, &["Casualty Two Probe"], paying(0, "Big Body"));
    assert_eq!(seen.offered, vec![true]);
    assert!(seen.most.is_empty(), "casualty N announces no X");
    assert_eq!(seen.sacrificable, vec![vec!["Big Body"]]);
    assert_eq!(copies(&game), 1);
}

#[test]
fn the_copy_that_isnt_legendary_with_starting_loyalty_x_becomes_such_a_token() {
    let mut game = deal(&["Casualty Walker", "Walker Mirror"], &["Big Body"]);
    let seen = play(&mut game, &["Casualty Walker", "Walker Mirror"], paying(2, "Big Body"));
    assert_eq!(seen.most, vec![MOST_POWER]);
    let walkers = on_battlefield(&game, "Casualty Walker");
    assert_eq!(walkers.len(), 3, "the legend rule leaves all three: neither copy is legendary");
    let card = walkers.iter().copied().find(|&id| !game.state.obj(id).token).expect("the original resolved");
    assert!(game.chars(card).supertypes.legendary);
    assert_eq!(game.state.obj(card).counters(CounterKind::Loyalty), 3);
    // The copy became a token (CR 707.10f), and the Mirror's copy of it copied its exceptions too (CR 707.9b).
    let tokens: Vec<ObjectId> = walkers.iter().copied().filter(|&id| game.state.obj(id).token).collect();
    assert_eq!(tokens.len(), 2);
    for token in tokens {
        assert!(!game.chars(token).supertypes.legendary, "a copy isn't legendary");
        assert_eq!(game.state.obj(token).counters(CounterKind::Loyalty), 2, "starting loyalty X");
    }
}
