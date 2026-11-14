//! Controlling another player (CR 723), on a table of test-only cards: Mindslaver's, Sorin Markov's and Worst Fears's "you
//! control target player during that player's next turn", and Emrakul, the Promised End's "..., After that turn, that player
//! takes an extra turn". These are:
//!
//! - the control applies to the next turn the player actually takes (CR 723.1), so a skipped turn leaves it waiting
//!   (723.1b), and it lasts through that turn's cleanup, ending as the next turn begins (723.1);
//! - every question asked of the controlled player in that turn is answered by the controller (723.5), who goes on making
//!   their own (723.8); the controlled player is still asked, is still the active player and keeps their objects (723.3);
//! - the costs are paid with the controlled player's resources (723.5a);
//! - the last such effect created is the one that works (723.1a), one on a player themselves gives no one control (723.9),
//!   and each of Emrakul's gives its extra turn after that turn, an overwritten one too (its ruling);
//! - the controller sees what the controlled player sees of the game - their hand, a face-down permanent they control
//!   (723.4) - and nothing of what they own outside the game, which they therefore can't have them choose (its ruling);
//! - a controller who leaves the game gives control back at once (CR 800.4a).

use std::collections::{HashSet, VecDeque};
use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::visibility::{knows_face, looks_into, observable};
use cantrip_engine::*;
use cantrip_mechanics::outside;

const P1: PlayerId = PlayerId(0);
const P2: PlayerId = PlayerId(1);
const P3: PlayerId = PlayerId(2);

fn a_player(_: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Player(_))
}

fn target_player(item: &StackItem) -> Option<PlayerId> {
    match item.target(0) {
        Target::Player(p) => Some(p),
        _ => None,
    }
}

fn control(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Some(p) = target_player(item) {
        g.control_next_turn(item, p);
    }
    Ok(())
}

fn control_then_extra(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Some(p) = target_player(item) {
        g.control_next_turn_then_extra_turn(item, p);
    }
    Ok(())
}

fn skip(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Some(p) = target_player(item) {
        g.skip_next_turns(item.object, p, 1);
    }
    Ok(())
}

fn lose_two(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Some(p) = target_player(item) {
        g.lose_life(p, 2);
    }
    Ok(())
}

fn wish(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    outside::may_put_card_into_hand(g, item.controller, item.object)?;
    Ok(())
}

const R: ManaCost = ManaCost {
    symbols: [0, 0, 0, 1, 0, 0],
    ..ManaCost::ZERO
};

/// An instant for {0}: "You control target player during that player's next turn." (Mindslaver's, Sorin Markov's.)
static SLAVER: CardDef = CardDef {
    name: "Probe Slaver",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[target(a_player)], control),
    ..CardDef::BLANK
};

/// An instant for {0}: "You gain control of target player during that player's next turn. After that turn, that player takes
/// an extra turn." (Emrakul, the Promised End's cast trigger.)
static PROMISE: CardDef = CardDef {
    name: "Probe Promise",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[target(a_player)], control_then_extra),
    ..CardDef::BLANK
};

/// An instant for {0}: "Target player skips their next turn."
static SKIP: CardDef = CardDef {
    name: "Probe Skip",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[target(a_player)], skip),
    ..CardDef::BLANK
};

/// An instant for {R}: "Target player loses 2 life."
static DRAIN: CardDef = CardDef {
    name: "Probe Drain",
    cost: R,
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[target(a_player)], lose_two),
    ..CardDef::BLANK
};

/// An instant for {0}: "You may put a card you own from outside the game into your hand." (North Wind Avatar's, Death Wish's.)
static WISH: CardDef = CardDef {
    name: "Probe Wish",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], wish),
    ..CardDef::BLANK
};

fn extra_turn(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.add_extra_turns(item.controller, 1);
    Ok(())
}

/// An instant for {0}: "Take an extra turn after this one."
static EXTRA: CardDef = CardDef { name: "Probe Extra", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[], extra_turn), ..CardDef::BLANK };

fn its_controllers_turn(_: &dyn Query, asker: &Asker, player: PlayerId) -> bool {
    player == asker.controller
}

fn nothing_more(_: &mut dyn Rules, _: &Asker) -> Result<(), Suspend> {
    Ok(())
}

const SKIP_TURN: Question = Question::new("SkipTurn", "skip this turn?", "", "Skip the turn", "Take the turn").no_default();

/// An artifact for {0}: "If you would begin your turn, you may skip that turn instead." (Time Vault's, without its tapping.)
static VAULT: CardDef = CardDef {
    name: "Probe Vault",
    types: TypeSet::of(&[CardType::Artifact]),
    turn_begin: &[TurnBeginRule { applies: its_controllers_turn, on_skip: nothing_more, question: &SKIP_TURN, hint: "you skip this turn" }],
    ..CardDef::BLANK
};

/// A 2/2 creature for {0} with enlist (CR 702.154).
static ENLISTER: CardDef = CardDef {
    name: "Probe Enlister",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    abilities: &[cantrip_mechanics::enlist::ENLIST],
    ..CardDef::BLANK
};

/// A 1/1 creature for {0}.
static BEAR: CardDef =
    CardDef { name: "Probe Bear", types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(1), ..CardDef::BLANK };

/// A basic Mountain: its mana ability is its type's (CR 305.6).
static MOUNTAIN: CardDef = CardDef {
    name: "Probe Mountain",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    supertypes: Supertypes {
        basic: true,
        legendary: false,
        snow: false,
        world: false,
    },
    subtypes: SubtypeSet::of(&[Subtype::Mountain]),
    ..CardDef::BLANK
};

/// A land with no mana ability, to fill hands and libraries.
static FILLER: CardDef = CardDef {
    name: "Filler",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        CardTable::new([&SLAVER, &PROMISE, &SKIP, &DRAIN, &WISH, &MOUNTAIN, &FILLER, &EXTRA, &VAULT, &ENLISTER, &BEAR])
    })
}

fn id(name: &str) -> CardId {
    table()
        .find(name)
        .unwrap_or_else(|| panic!("no card {name}"))
}

/// A game, Bot1 first, libraries unshuffled: each hand the named cards, then Fillers; each sideboard the named cards.
fn deal(hands: &[&[&str]], sideboards: &[&[&str]]) -> Game {
    let decks = hands
        .iter()
        .map(|names| {
            let mut deck: Vec<CardId> = names.iter().map(|n| id(n)).collect();
            deck.extend(std::iter::repeat(id("Filler")).take(30 - deck.len()));
            deck
        })
        .collect();
    let sideboards = sideboards
        .iter()
        .map(|names| names.iter().map(|n| id(n)).collect())
        .collect();
    let mut game = Game::new(
        table(),
        &Setup {
            decks,
            sideboards,
            seed: 1,
            shuffle: false,
            opening_chooser: None,
            ..Default::default()
        },
    );
    game.verify = true;
    game
}

/// What a player does on their own turn, or on another's: in turn `turn`, at their first priority of its precombat main phase
/// with the stack empty, play the land or cast the spell named (at the player named, for a target).
#[derive(Clone, Copy)]
struct Move {
    player: PlayerId,
    turn: u32,
    card: &'static str,
    at: Option<PlayerId>,
}

const fn cast(player: PlayerId, turn: u32, card: &'static str, at: Option<PlayerId>) -> Move {
    Move {
        player,
        turn,
        card,
        at,
    }
}

/// One question asked: the turn, its active player, who it was asked of, and who answered it.
#[derive(Clone, Debug, PartialEq)]
struct Asked {
    turn: u32,
    active: PlayerId,
    step: Step,
    player: PlayerId,
    by: PlayerId,
    kind: &'static str,
}

fn kind_name(kind: &RequestKind) -> &'static str {
    match kind {
        RequestKind::Priority { .. } => "Priority",
        RequestKind::DeclareAttackers { .. } => "DeclareAttackers",
        RequestKind::DeclareBlockers { .. } => "DeclareBlockers",
        RequestKind::DiscardToHandSize { .. } => "DiscardToHandSize",
        RequestKind::PayCosts { .. } => "PayCosts",
        RequestKind::ChooseCards { .. } => "ChooseCards",
        RequestKind::May { .. } => "May",
        RequestKind::ChooseReplacement { .. } => "ChooseReplacement",
        _ => "Other",
    }
}

/// Plays on, making the moves in order and passing otherwise, until `until` holds at a question (or the game ends), and
/// returns every question asked on the way, with the turn it was asked in and who answered it.
fn play(game: &mut Game, moves: &[Move], mut until: impl FnMut(&Game) -> bool) -> Vec<Asked> {
    let mut plan: VecDeque<Move> = moves.iter().copied().collect();
    let mut asked = Vec::new();
    for _ in 0..2000 {
        game.run();
        if game.is_over() || until(game) {
            return asked;
        }
        let request = game.pending().cloned().expect("a question");
        let by = game.state.player_controller(request.player);
        let s = &game.state;
        asked.push(Asked {
            turn: s.turn.number,
            active: s.turn.active,
            step: s.turn.step(),
            player: request.player,
            by,
            kind: kind_name(&request.kind),
        });
        let answer = match &request.kind {
            RequestKind::Priority { options } => {
                let due = plan.front().copied().filter(|m| {
                    m.player == request.player
                        && m.turn == s.turn.number
                        && s.turn.step() == Step::PrecombatMain
                        && s.stack.is_empty()
                });
                match due {
                    Some(m) => {
                        plan.pop_front();
                        let option = options
                            .iter()
                            .find(|o| {
                                matches!(o.kind, ActionKind::CastSpell | ActionKind::PlayLand)
                                    && o.source.is_some_and(|c| game.name(c) == m.card)
                            })
                            .unwrap_or_else(|| {
                                panic!(
                                    "{} is not offered to {:?} in turn {}",
                                    m.card, request.player, s.turn.number
                                )
                            });
                        let mut action = Action::of(option);
                        if let Some(p) = m.at {
                            action.targets = vec![Target::Player(p)];
                        }
                        Answer::Act(action)
                    }
                    None => Answer::Pass,
                }
            }
            // Pay with a mana ability while one is offered, else take the pool's way.
            RequestKind::PayCosts { options, .. } => {
                let pick = options
                    .iter()
                    .find(|o| o.kind == ActionKind::ActivateAbility)
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::AssignMana))
                    .or_else(|| {
                        options
                            .iter()
                            .find(|o| o.kind == ActionKind::ConfirmCasting)
                    })
                    .unwrap_or_else(|| panic!("no way to pay: {options:?}"));
                Answer::Act(Action::of(pick))
            }
            RequestKind::DiscardToHandSize { count, hand } => {
                Answer::Cards(hand.iter().copied().take(*count as usize).collect())
            }
            // A Probe Enlister that may attack attacks the first player it may; nothing else does.
            RequestKind::DeclareAttackers { eligible, players, .. } => Answer::Attack(
                eligible
                    .iter()
                    .filter(|&&c| game.name(c) == "Probe Enlister")
                    .filter_map(|&c| players.first().map(|&p| (c, Target::Player(p))))
                    .collect(),
            ),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            RequestKind::ChooseCards {
                source,
                cards,
                rule,
                ..
            } => Answer::Cards(
                cantrip_api::select::first_answer(game, rule, request.player, *source, cards, true)
                    .unwrap_or_default(),
            ),
            RequestKind::May { .. } => Answer::Yes,
            RequestKind::ChooseReplacement { .. } => Answer::Index(0),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!(
        "the game did not reach the position by turn {}",
        game.state.turn.number
    );
}

/// Who the history says came to control which player, in order (`Event::PlayerControlled`).
fn control_changes(game: &Game) -> Vec<(PlayerId, PlayerId)> {
    game.log
        .iter()
        .filter_map(|r| match r.event {
            Event::PlayerControlled { player, controller } => Some((player, controller)),
            _ => None,
        })
        .collect()
}

fn turn_is(n: u32) -> impl FnMut(&Game) -> bool {
    move |g: &Game| g.state.turn.number == n && g.state.turn.step() == Step::PrecombatMain
}

/// The active player of each turn from the first to `last`, from the questions asked.
fn takers(asked: &[Asked], last: u32) -> Vec<PlayerId> {
    (1..=last)
        .map(|t| {
            asked
                .iter()
                .find(|a| a.turn == t)
                .unwrap_or_else(|| panic!("nothing asked in turn {t}"))
                .active
        })
        .collect()
}

/// Who answered the questions asked of `player` in turn `turn`.
fn answerers(asked: &[Asked], turn: u32, player: PlayerId) -> HashSet<PlayerId> {
    asked
        .iter()
        .filter(|a| a.turn == turn && a.player == player)
        .map(|a| a.by)
        .collect()
}

/// CR 723.1, 723.5, 723.8, 723.3: Bot1 controls Bot2 during Bot2's next turn, turn 2. Every question asked of Bot2 then -
/// priority, the discard to hand size in its cleanup - Bot1 answers, while Bot1 still answers its
/// own; Bot2 is still the active player. Before that turn (after the effect is made) and from turn 3 on, Bot2 answers for itself.
#[test]
fn the_controller_answers_the_controlled_players_every_question_through_its_cleanup() {
    let mut game = deal(&[&["Probe Slaver"], &[]], &[]);
    let asked = play(
        &mut game,
        &[cast(P1, 1, "Probe Slaver", Some(P2))],
        turn_is(3),
    );
    assert_eq!(takers(&asked, 3), [P1, P2, P1]);
    // Made in turn 1, it waits: Bot2 still answered its own priority after it resolved.
    assert_eq!(answerers(&asked, 1, P2), HashSet::from([P2]));
    assert_eq!(answerers(&asked, 2, P2), HashSet::from([P1]));
    assert_eq!(
        answerers(&asked, 2, P1),
        HashSet::from([P1]),
        "the controller goes on making its own decisions"
    );
    let kinds: HashSet<&str> = asked
        .iter()
        .filter(|a| a.turn == 2 && a.player == P2)
        .map(|a| a.kind)
        .collect();
    assert!(
        kinds.contains("Priority") && kinds.contains("DiscardToHandSize"),
        "{kinds:?}"
    );
    assert!(
        asked
            .iter()
            .any(|a| a.turn == 2 && a.player == P2 && a.step == Step::Cleanup && a.by == P1),
        "the control lasts through the turn's cleanup"
    );
    // Turn 3 has begun: the control is over, and nothing waits.
    assert_eq!(game.state.player_controller(P2), P2);
    assert!(!game.state.effects.iter().any(|e| matches!(
        e.kind,
        EffectKind::ControlNextTurn { .. } | EffectKind::ControlsPlayer(_)
    )));
    play(&mut game, &[], turn_is(4));
    assert_eq!(game.state.player_controller(P2), P2, "only the next turn");
}

/// CR 723.1b: a skipped turn is not the turn the control is for. Bot2 skips its next turn and is controlled during its next
/// turn: turn 2 is Bot1's again (Bot2's was skipped), and Bot1 controls Bot2 in turn 3, the next turn Bot2 actually takes.
#[test]
fn a_skipped_turn_leaves_the_control_waiting_for_the_next_turn_taken() {
    let mut game = deal(&[&["Probe Slaver", "Probe Skip"], &[]], &[]);
    let moves = [
        cast(P1, 1, "Probe Slaver", Some(P2)),
        cast(P1, 1, "Probe Skip", Some(P2)),
    ];
    let asked = play(&mut game, &moves, turn_is(4));
    assert_eq!(takers(&asked, 4), [P1, P1, P2, P1]);
    assert_eq!(
        answerers(&asked, 2, P2),
        HashSet::from([P2]),
        "still waiting through the skipped turn"
    );
    assert_eq!(answerers(&asked, 3, P2), HashSet::from([P1]));
    assert_eq!(answerers(&asked, 4, P2), HashSet::from([P2]));
}

/// CR 723.1a, 723.9: the last effect created is the one that works. Bot1 makes Bot2 controlled during its next turn, and then
/// Bot2 makes itself controlled by itself: Bot2 makes its own decisions in turn 2. In the other order Bot1 controls it.
#[test]
fn the_last_control_created_is_the_one_that_works() {
    let mut game = deal(&[&["Probe Slaver"], &["Probe Slaver"]], &[]);
    let moves = [
        cast(P1, 1, "Probe Slaver", Some(P2)),
        cast(P2, 1, "Probe Slaver", Some(P2)),
    ];
    let asked = play(&mut game, &moves, turn_is(3));
    assert_eq!(answerers(&asked, 2, P2), HashSet::from([P2]));

    // Bot2's first, then Bot1's: Bot1 controls Bot2.
    let mut game = deal(&[&["Probe Slaver"], &["Probe Slaver"]], &[]);
    let moves = [
        cast(P2, 1, "Probe Slaver", Some(P2)),
        cast(P1, 1, "Probe Slaver", Some(P2)),
    ];
    let asked = play(&mut game, &moves, turn_is(3));
    assert_eq!(answerers(&asked, 2, P2), HashSet::from([P1]));
}

/// Emrakul, the Promised End's rulings: each effect gives its extra turn after the controlled turn, the one the other
/// overwrote too; after a skipped turn, the extra turn comes after the turn controlled. Two of them on Bot2, then Bot2 skips
/// its next turn: turn 2 is Bot1's (Bot2's skipped), Bot2 is controlled in turn 3 and takes turns 4 and 5 for itself.
#[test]
fn each_extra_turn_follows_the_controlled_turn_however_long_it_waits() {
    let mut game = deal(
        &[&["Probe Promise", "Probe Promise", "Probe Skip"], &[]],
        &[],
    );
    let moves = [
        cast(P1, 1, "Probe Promise", Some(P2)),
        cast(P1, 1, "Probe Promise", Some(P2)),
        cast(P1, 1, "Probe Skip", Some(P2)),
    ];
    let asked = play(&mut game, &moves, turn_is(6));
    assert_eq!(takers(&asked, 6), [P1, P1, P2, P2, P2, P1]);
    assert_eq!(answerers(&asked, 3, P2), HashSet::from([P1]));
    assert_eq!(answerers(&asked, 4, P2), HashSet::from([P2]));
    assert_eq!(answerers(&asked, 5, P2), HashSet::from([P2]));
}

/// CR 723.5a: the controller pays the controlled player's costs with that player's resources. In Bot2's controlled turn Bot1
/// has Bot2 play Bot2's Mountain and cast Bot2's Probe Drain at Bot2: Bot2's Mountain pays, and Bot1's stays untapped.
#[test]
fn the_controlled_players_resources_pay_their_costs() {
    let mut game = deal(
        &[
            &["Probe Slaver", "Probe Mountain"],
            &["Probe Mountain", "Probe Drain"],
        ],
        &[],
    );
    let moves = [
        cast(P1, 1, "Probe Mountain", None),
        cast(P1, 1, "Probe Slaver", Some(P2)),
        cast(P2, 2, "Probe Mountain", None),
        cast(P2, 2, "Probe Drain", Some(P2)),
    ];
    let asked = play(&mut game, &moves, turn_is(3));
    assert!(
        asked
            .iter()
            .any(|a| a.turn == 2 && a.player == P2 && a.kind == "PayCosts" && a.by == P1)
    );
    assert_eq!(game.state.player(P2).life, 18);
    let mountain = |p: PlayerId| {
        game.state
            .battlefield()
            .iter()
            .copied()
            .find(|&id| game.name(id) == "Probe Mountain" && game.state.obj(id).controller == p)
    };
    let theirs = mountain(P2).expect("Bot2's Mountain was played");
    assert_eq!(game.state.obj(theirs).owner, P2);
    assert!(
        game.log
            .iter()
            .any(|r| matches!(r.event, Event::Tapped { permanent, .. } if permanent == theirs)),
        "Bot2's Mountain paid"
    );
    let ours = mountain(P1).expect("Bot1's Mountain");
    assert!(
        !game
            .log
            .iter()
            .any(|r| matches!(r.event, Event::Tapped { permanent, .. } if permanent == ours)),
        "Bot1's Mountain did not"
    );
    assert!(
        game.state
            .zone(Zone::graveyard(P2))
            .iter()
            .any(|&c| game.name(c) == "Probe Drain")
    );
}

/// CR 723.4: while Bot1 controls Bot2, Bot1 sees Bot2's hand and the face of a card Bot2 exiled face down and may look at, but not the
/// cards Bot2 owns outside the game; before and after, none of it.
#[test]
fn the_controller_sees_what_the_controlled_player_sees_of_the_game() {
    let mut game = deal(&[&["Probe Slaver"], &[]], &[&[], &["Probe Wish"]]);
    // A card Bot2 exiled face down and may look at (a foretold card, CR 406.3, 702.143a), put there by hand before the game
    // begins.
    let face_down = {
        let new = game
            .state
            .push_obj(Obj::new(id("Filler"), P2, Zone::exile(P2)));
        game.state.update_obj(new, |o| {
            o.face_down = true;
            o.seen_by = 1 << P2.0;
        });
        game.state.zone_push(Zone::exile(P2), new);
        new
    };
    let waiting = |g: &Game| {
        g.state
            .effects
            .iter()
            .any(|e| matches!(e.kind, EffectKind::ControlNextTurn { .. }))
    };
    play(&mut game, &[cast(P1, 1, "Probe Slaver", Some(P2))], waiting);
    let sees = |game: &Game| {
        let s = &game.state;
        let visible = observable(game, P1, &HashSet::new());
        let hand = s.zone(Zone::hand(P2)).iter().all(|c| visible.contains(c));
        let sideboard = s
            .zone(Zone::sideboard(P2))
            .iter()
            .any(|c| visible.contains(c));
        assert_eq!(hand, looks_into(s, P1, Zone::hand(P2)));
        assert!(
            !looks_into(s, P1, Zone::sideboard(P2)),
            "outside the game is the player's alone"
        );
        (hand, knows_face(s, P1, face_down), sideboard)
    };
    assert_eq!(
        sees(&game),
        (false, false, false),
        "the effect waits: nothing is shown yet"
    );
    play(&mut game, &[], turn_is(2));
    assert_eq!(game.state.player_controller(P2), P1);
    assert_eq!(sees(&game), (true, true, false));
    assert!(
        !looks_into(&game.state, P2, Zone::hand(P1)),
        "the controlled player sees no more of the controller's"
    );
    play(&mut game, &[], turn_is(3));
    assert_eq!(sees(&game), (false, false, false));
}

/// Emrakul, the Promised End's ruling: "If an effect instructs that player to choose a card from outside the game, you can't
/// have that player choose any card." Bot2's Probe Wish, cast in Bot2's controlled turn, brings nothing in; cast in Bot2's own
/// turn, it brings in the card.
#[test]
fn a_controlled_player_chooses_nothing_from_outside_the_game() {
    let mut game = deal(
        &[&["Probe Slaver"], &["Probe Wish", "Probe Wish"]],
        &[&[], &["Probe Drain"]],
    );
    let moves = [
        cast(P1, 1, "Probe Slaver", Some(P2)),
        cast(P2, 2, "Probe Wish", None),
    ];
    play(&mut game, &moves, turn_is(3));
    let in_hand = |game: &Game| {
        game.state
            .zone(Zone::hand(P2))
            .iter()
            .any(|&c| game.name(c) == "Probe Drain")
    };
    assert!(!in_hand(&game));
    assert_eq!(game.state.zone(Zone::sideboard(P2)).len(), 1);
    play(&mut game, &[cast(P2, 4, "Probe Wish", None)], turn_is(5));
    assert!(in_hand(&game), "in its own turn it chooses for itself");
}

/// CR 800.4a: the effects that give a player control of players end as they leave the game. In a game of three, Bot1 controls
/// Bot2 in turn 2 and concedes during it: Bot2 answers for itself from then on.
#[test]
fn a_controller_leaving_the_game_gives_the_player_back() {
    let mut game = deal(&[&["Probe Slaver"], &[], &[]], &[]);
    play(
        &mut game,
        &[cast(P1, 1, "Probe Slaver", Some(P2))],
        turn_is(2),
    );
    assert_eq!(game.state.player_controller(P2), P1);
    assert_eq!(control_changes(&game), [(P2, P1)]);
    game.concede(P1);
    assert_eq!(game.state.player_controller(P2), P2);
    assert_eq!(control_changes(&game), [(P2, P1), (P2, P2)], "the history says the control ended");
    assert!(
        !game
            .state
            .effects
            .iter()
            .any(|e| matches!(e.kind, EffectKind::ControlsPlayer(_)))
    );
    let asked = play(&mut game, &[], |g| g.state.turn.number == 3);
    assert_eq!(answerers(&asked, 2, P2), HashSet::from([P2]));
    assert!(game.state.in_game(P3));
}

/// CR 723.1, 614.10, 500.11: a turn that is skipped never begins, so the control of the turn before still holds while the
/// skip is decided. Bot1 controls Bot2 in turn 2 and has Bot2 cast Probe Vault and take an extra turn; as that extra turn
/// would begin, Bot2's "you may skip that turn" is answered by Bot1 (it skips it), and turn 3 is Bot1's.
#[test]
fn a_skip_decided_as_the_next_turn_would_begin_is_the_controllers() {
    let mut game = deal(&[&["Probe Slaver"], &["Probe Vault", "Probe Extra"]], &[]);
    let moves = [cast(P1, 1, "Probe Slaver", Some(P2)), cast(P2, 2, "Probe Vault", None), cast(P2, 2, "Probe Extra", None)];
    let asked = play(&mut game, &moves, turn_is(3));
    assert_eq!(game.state.turn.active, P1, "Bot2's extra turn was skipped");
    let skip = asked.iter().find(|a| a.kind == "May" && a.player == P2).expect("Bot2 was asked to skip its extra turn");
    assert_eq!(skip.by, P1);
    assert_eq!(game.state.player_controller(P2), P2, "the control ended as turn 3 began");
    assert_eq!(control_changes(&game), [(P2, P1), (P2, P2)], "the history says when the control began and ended");
}

/// The same with an effect that must skip the turn beside the one that may (CR 616.1): which applies first is chosen by Bot2,
/// so by Bot1.
#[test]
fn the_order_of_skips_as_the_next_turn_would_begin_is_the_controllers() {
    let mut game = deal(&[&["Probe Slaver"], &["Probe Vault", "Probe Extra", "Probe Skip"]], &[]);
    let moves = [
        cast(P1, 1, "Probe Slaver", Some(P2)),
        cast(P2, 2, "Probe Vault", None),
        cast(P2, 2, "Probe Extra", None),
        cast(P2, 2, "Probe Skip", Some(P2)),
    ];
    let asked = play(&mut game, &moves, turn_is(3));
    let order = asked.iter().find(|a| a.kind == "ChooseReplacement" && a.player == P2).expect("Bot2 ordered the skips");
    assert_eq!(order.by, P1);
}

/// CR 723.1a, 800.4a: the last effect created whose controller is still in the game works. In a game of three Bot1 and then
/// Bot2 make Bot3 controlled during its next turn, and Bot2 concedes before it: Bot1 controls Bot3 in it.
#[test]
fn a_leavers_control_gives_way_to_the_one_before_it() {
    let mut game = deal(&[&["Probe Slaver"], &["Probe Slaver"], &[]], &[]);
    let moves = [cast(P1, 1, "Probe Slaver", Some(P3)), cast(P2, 1, "Probe Slaver", Some(P3))];
    let both = |g: &Game| g.state.stack.is_empty() && g.state.effects.iter().filter(|e| matches!(e.kind, EffectKind::ControlNextTurn { .. })).count() == 2;
    play(&mut game, &moves, both);
    game.concede(P2);
    play(&mut game, &[], |g| g.state.turn.active == P3 && g.state.turn.step() == Step::PrecombatMain);
    let turn = game.state.turn.number;
    assert_eq!(game.state.player_controller(P3), P1);
    let asked = play(&mut game, &[], |g| g.state.turn.number > turn);
    assert_eq!(answerers(&asked, turn, P3), HashSet::from([P1]));
}

/// CR 723.1a, 800.4a: Bot2's control of Bot3, the later, works until Bot2 concedes during Bot3's turn; Bot1's, which it
/// overwrote, works for the rest of that turn.
#[test]
fn a_controller_leaving_during_the_turn_leaves_it_to_the_one_before() {
    let mut game = deal(&[&["Probe Slaver"], &["Probe Slaver"], &[]], &[]);
    let moves = [cast(P1, 1, "Probe Slaver", Some(P3)), cast(P2, 1, "Probe Slaver", Some(P3))];
    play(&mut game, &moves, |g| g.state.turn.active == P3 && g.state.turn.step() == Step::PrecombatMain);
    assert_eq!(game.state.player_controller(P3), P2);
    let turn = game.state.turn.number;
    game.concede(P2);
    assert_eq!(game.state.player_controller(P3), P1);
    assert_eq!(control_changes(&game), [(P3, P2), (P3, P1)], "the history says who controls Bot3 after Bot2 left");
    let asked = play(&mut game, &[], |g| g.state.turn.number > turn);
    assert_eq!(answerers(&asked, turn, P3), HashSet::from([P1]));
}

/// Judgement call (CR 800.4a): an Emrakul effect whose only controller leaves before the turn gives no control, but the
/// extra turn after it is not the leaver's and is still taken.
#[test]
fn a_leavers_effect_gives_no_control_but_its_extra_turn() {
    let mut game = deal(&[&[], &["Probe Promise"], &[]], &[]);
    let waiting = |g: &Game| g.state.stack.is_empty() && g.state.effects.iter().any(|e| matches!(e.kind, EffectKind::ControlNextTurn { .. }));
    play(&mut game, &[cast(P2, 1, "Probe Promise", Some(P3))], waiting);
    game.concede(P2);
    let asked = play(&mut game, &[], |g| g.state.turn.active == P1 && g.state.turn.number > 1);
    let turns: Vec<(u32, PlayerId)> = {
        let mut t: Vec<(u32, PlayerId)> = asked.iter().map(|a| (a.turn, a.active)).collect();
        t.dedup();
        t
    };
    let third: Vec<u32> = turns.iter().filter(|(_, p)| *p == P3).map(|(n, _)| *n).collect();
    assert_eq!(third.len(), 2, "Bot3 takes its turn and the extra one after it: {turns:?}");
    for n in third {
        assert_eq!(answerers(&asked, n, P3), HashSet::from([P3]));
    }
}

/// CR 723.5, 508.1g, 702.154a: the optional costs to attack of a controlled player's creatures are that player's decisions,
/// so the controller's. Bot2 casts Probe Enlister and Probe Bear in turn 2; Bot1 controls Bot2 in turn 4, has the Enlister
/// attack and enlist the Bear - asked of Bot2, answered by Bot1 - and the Bear is tapped for it.
#[test]
fn a_controlled_players_enlist_is_the_controllers() {
    let mut game = deal(&[&["Filler", "Filler", "Probe Slaver"], &["Probe Enlister", "Probe Bear"]], &[]);
    let moves = [cast(P2, 2, "Probe Enlister", None), cast(P2, 2, "Probe Bear", None), cast(P1, 3, "Probe Slaver", Some(P2))];
    let asked = play(&mut game, &moves, turn_is(5));
    let enlist = asked.iter().find(|a| a.turn == 4 && a.kind == "ChooseCards" && a.player == P2).expect("Bot2 was asked whom to enlist");
    assert_eq!(enlist.by, P1);
    assert!(game.log.iter().any(|r| matches!(r.event, Event::PaidToAttack { option: AttackOption::Enlist, .. })), "the Bear was enlisted");
}
