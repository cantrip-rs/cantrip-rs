//! Eluge, the Shoreless Sea where no printed card in the engine takes it: its power and toughness as a spell another player
//! than its owner controls ("you" is its controller, CR 109.5, on the stack too), and its cost reduction's "land" for a flooded
//! permanent that has stopped being a land. Test cards do it, beside the printed Eluge and every other card the engine has.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

fn spell_on_stack(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.state().stack.iter().any(|i| i.kind == StackKind::Spell && i.object == id))
}

/// "Gain control of target spell."
fn commandeer(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(spell) = item.target(0) {
        g.change_controller(spell, item.controller);
    }
    Ok(())
}

fn land(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Land))
}

/// "Target land becomes an artifact and loses its other card types."
fn unland(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(id) = item.target(0) {
        g.add_effect(item.object, id, EffectKind::SetTypes(TypeSet::of(&[CardType::Artifact])), Duration::Indefinite);
    }
    Ok(())
}

fn creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

/// "Target creature gets +0/+2 and loses all abilities."
fn mute(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(id) = item.target(0) {
        // "Target creature gets +0/+2 and loses all abilities": one instruction.
        cantrip_api::instruction(g, |g| {
            g.add_effect(item.object, id, EffectKind::Pump(0, 2), Duration::Indefinite);
            g.add_effect(item.object, id, EffectKind::LoseAllAbilities, Duration::Indefinite);
        });
    }
    Ok(())
}

static MUTE: CardDef = CardDef { name: "Probe Mute", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[target(creature)], mute), ..CardDef::BLANK };
static COMMANDEER: CardDef = CardDef { name: "Probe Commandeer", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[target(spell_on_stack)], commandeer), ..CardDef::BLANK };
static UNLAND: CardDef = CardDef { name: "Probe Unland", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[target(land)], unland), ..CardDef::BLANK };
/// An instant costing {4}{U} that does nothing: what Eluge's reduction is seen on.
static OPT: CardDef = CardDef {
    name: "Probe Opt",
    cost: ManaCost { generic: 4, symbols: [0, 1, 0, 0, 0, 0], ..ManaCost::ZERO },
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], |_, _| Ok(())),
    ..CardDef::BLANK
};

/// "Put a flood counter on each land you control."
fn flood_all(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    for id in g.permanents_of(item.controller) {
        if g.chars(id).is(CardType::Land) {
            g.add_counters(id, CounterKind::Flood, 1);
        }
    }
    Ok(())
}

static FLOOD_ALL: CardDef = CardDef { name: "Probe Flood All", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], flood_all), ..CardDef::BLANK };
/// Mind Spring at X = 254: an instant costing {254}{U}{U}.
static SPRING: CardDef = CardDef {
    name: "Probe Spring",
    cost: ManaCost { generic: 254, symbols: [0, 2, 0, 0, 0, 0], ..ManaCost::ZERO },
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], |_, _| Ok(())),
    ..CardDef::BLANK
};
/// Expansion's {U/R}{U/R}.
static EXPANSION: CardDef = CardDef {
    name: "Probe Expansion",
    cost: ManaCost { hybrid: [Some((Mana::U, Mana::R)), Some((Mana::U, Mana::R)), None, None, None, None], ..ManaCost::ZERO },
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], |_, _| Ok(())),
    ..CardDef::BLANK
};

/// Unnerving Assault's {U/R} alone: an instant costing one hybrid symbol.
static ASSAULT: CardDef = CardDef {
    name: "Probe Assault",
    cost: ManaCost { hybrid: [Some((Mana::U, Mana::R)), None, None, None, None, None], ..ManaCost::ZERO },
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], |_, _| Ok(())),
    ..CardDef::BLANK
};
/// {U/R}{U/B}: two hybrid symbols of different pairs, each with a blue half.
static MIXED: CardDef = CardDef {
    name: "Probe Mixed",
    cost: ManaCost { hybrid: [Some((Mana::U, Mana::R)), Some((Mana::U, Mana::B)), None, None, None, None], ..ManaCost::ZERO },
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], |_, _| Ok(())),
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&COMMANDEER, &UNLAND, &OPT, &MUTE, &FLOOD_ALL, &SPRING, &EXPANSION, &ASSAULT, &MIXED])))
}

/// What a seat does, in order, each as soon as its turn has come and it is offered: play a land, or cast a card (aimed at a
/// permanent named, or at the spell on top of the stack).
enum Step {
    Play(usize, u32, &'static str),
    Cast(usize, u32, &'static str, Option<&'static str>),
}

/// A game of the two hands (then Islands), played until every step is done, turn `until` has come and the stack is empty. Every question is shown to
/// `watch` first. Mana is paid with Moxen, then lands.
fn play(hands: [&[&str]; 2], steps: Vec<Step>, until: u32, mut watch: impl FnMut(&Game, &Request)) -> Game {
    let names = |list: &[&str]| -> Vec<CardId> { list.iter().map(|n| table().find(n).unwrap_or_else(|| panic!("no card {n}"))).collect() };
    let deck = |hand: &[&str]| {
        let mut deck = names(hand);
        deck.extend(names(&vec!["Island"; 30 - hand.len()]));
        deck
    };
    let mut game = Game::new(table(), &Setup { decks: vec![deck(hands[0]), deck(hands[1])], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut steps = steps.into_iter().peekable();
    for _ in 0..3000 {
        game.run();
        if steps.peek().is_none() && game.state.turn.number >= until && game.state.stack.is_empty() && game.state.triggers.is_empty() {
            return game;
        }
        let Some(request) = game.pending().cloned() else { panic!("the game ended on turn {} with {:?} left: {:?}", game.state.turn.number, steps.peek().map(|s| match s { Step::Play(_, _, n) | Step::Cast(_, _, n, _) => *n }), game.state.outcome) };
        watch(&game, &request);
        let seat = request.player.0 as usize;
        let turn = game.state.turn.number;
        let answer = match &request.kind {
            RequestKind::Priority { options } => {
                let option = match steps.peek() {
                    Some(&Step::Play(s, at, name)) if s == seat && turn >= at => {
                        options.iter().find(|o| o.kind == ActionKind::PlayLand && o.source.is_some_and(|c| game.name(c) == name))
                    }
                    Some(&Step::Cast(s, at, name, _)) if s == seat && turn >= at => {
                        options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|c| game.name(c) == name))
                    }
                    _ => None,
                };
                match option {
                    Some(option) => {
                        let aimed = match steps.next() {
                            Some(Step::Cast(_, _, _, aimed)) => aimed,
                            _ => None,
                        };
                        let mut action = Action::of(option);
                        if let Some(aimed) = aimed
                            && option.targets_required > 0
                        {
                            let target = if aimed == "stack" {
                                Target::Object(game.state.stack.last().expect("a spell").object)
                            } else if aimed == "flooded" {
                                Target::Object(*game.state.battlefield().iter().find(|&&id| game.state.obj(id).counters(CounterKind::Flood) > 0).expect("a flooded land"))
                            } else {
                                Target::Object(*game.state.battlefield().iter().find(|&&id| game.name(id) == aimed).expect("the target"))
                            };
                            action.targets = vec![target];
                        }
                        Answer::Act(action)
                    }
                    None => Answer::Pass,
                }
            }
            RequestKind::PayCosts { options, .. } => {
                let mox = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s).text().starts_with("Mox")));
                let land = options.iter().find(|o| o.kind == ActionKind::ActivateAbility);
                let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting);
                Answer::Act(Action::of(confirm.or(mox).or(land).expect("mana to pay with, or the payment to confirm")))
            }
            RequestKind::ChooseTarget { candidates, .. } => Answer::Targets(vec![candidates[0]]),
            RequestKind::OrderTriggers { sources, .. } => Answer::Order((0..count_u32(sources.len())).collect()),
            // The newest cards, so a card planned for a later turn stays.
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().rev().copied().take(*count as usize).collect()),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the game went on too long");
}

/// Bot1 (one Island) casts Eluge on turn 5; Bot2 (three Islands) takes the spell with Probe Commandeer. On the stack under
/// Bot2 it counts Bot2's Islands (CR 109.5: "you" is its controller), a 3/3, and it enters under Bot2 a 3/3.
#[test]
fn eluge_as_a_spell_another_player_controls_counts_that_players_islands() {
    let mut on_stack: Vec<(PlayerId, i16)> = Vec::new();
    let game = play(
        [&["Island", "Mox Sapphire", "Mox Sapphire", "Mox Sapphire", "Eluge, the Shoreless Sea"], &["Island", "Island", "Island", "Probe Commandeer"]],
        vec![
            Step::Play(0, 1, "Island"),
            Step::Cast(0, 1, "Mox Sapphire", None),
            Step::Cast(0, 1, "Mox Sapphire", None),
            Step::Cast(0, 1, "Mox Sapphire", None),
            Step::Play(1, 2, "Island"),
            Step::Play(1, 4, "Island"),
            Step::Play(1, 6, "Island"),
            Step::Cast(0, 7, "Eluge, the Shoreless Sea", None),
            Step::Cast(1, 7, "Probe Commandeer", Some("stack")),
        ],
        7,
        |game, _| {
            if let Some(item) = game.state.stack.iter().find(|i| i.kind == StackKind::Spell && game.name(i.object) == "Eluge, the Shoreless Sea") {
                let seen = (item.controller, game.chars(item.object).power.unwrap_or(-1));
                if on_stack.last() != Some(&seen) {
                    on_stack.push(seen);
                }
            }
        },
    );
    assert_eq!(on_stack, [(PlayerId(0), 1), (PlayerId(1), 3)], "Eluge on the stack, under Bot1 and then under Bot2");
    let eluge = *game.state.battlefield().iter().find(|&&id| game.name(id) == "Eluge, the Shoreless Sea").expect("Eluge");
    assert_eq!((game.state.obj(eluge).controller, game.chars(eluge).power), (PlayerId(1), Some(3)));
}

/// Eluge's flood counter is on Bot1's first Island, so the first instant Bot1 would cast each turn, Probe Opt ({4}{U}), costs {U}
/// less: {4}. On turn 4 Bot2's Probe Unland makes the flooded Island an artifact - still flooded, but no land (Eluge lives on as a
/// 1/1 with Bot1's second Island) - so on turn 5 the reduction is none and the Opt costs {4}{U} (CR 601.2f): its total as the
/// table works it out for the cast, each time Bot1 has priority with Eluge out and nothing on the stack.
#[test]
fn eluges_reduction_counts_no_flooded_permanent_that_is_no_land() {
    let mut totals: Vec<u32> = Vec::new();
    play(
        [&["Island", "Mox Sapphire", "Mox Sapphire", "Mox Sapphire", "Eluge, the Shoreless Sea", "Probe Opt", "Island"], &["Probe Unland"]],
        vec![
            Step::Play(0, 1, "Island"),
            Step::Cast(0, 1, "Mox Sapphire", None),
            Step::Cast(0, 1, "Mox Sapphire", None),
            Step::Cast(0, 1, "Mox Sapphire", None),
            Step::Cast(0, 1, "Eluge, the Shoreless Sea", None),
            Step::Play(0, 3, "Island"),
            Step::Cast(1, 4, "Probe Unland", Some("flooded")),
        ],
        6,
        |game, request| {
            let eluge = game.state.battlefield().iter().any(|&id| game.name(id) == "Eluge, the Shoreless Sea");
            if request.player != PlayerId(0) || !eluge || !game.state.stack.is_empty() || !matches!(request.kind, RequestKind::Priority { .. }) {
                return;
            }
            let Some(&opt) = game.state.zone(Zone::hand(PlayerId(0))).iter().find(|&&c| game.name(c) == "Probe Opt") else {
                panic!("no Opt on turn {}: hand {:?}, graveyard {:?}", game.state.turn.number, game.state.zone(Zone::hand(PlayerId(0))).iter().map(|&c| game.name(c)).collect::<Vec<_>>(), game.state.zone(Zone::graveyard(PlayerId(0))).iter().map(|&c| game.name(c)).collect::<Vec<_>>())
            };
            let total = cast_total(game, PlayerId(0), &ActionOption::on(ActionKind::CastSpell, opt, 0)).map_or(5, |(_, total, _)| total.mana_value());
            if totals.last() != Some(&total) {
                totals.push(total);
            }
        },
    );
    assert_eq!(totals, [4, 5], "the Opt's total before the Island stopped being a land, and after");
}

/// Eluge's power and toughness are stars, defined by its ability (CR 208.2a); once it has lost all its abilities they are 0
/// (CR 208.5, 613.1f): with Probe Mute's +0/+2 it is a 0/2, not a creature with no power.
#[test]
fn eluge_that_lost_its_abilities_has_power_0() {
    let game = play(
        [&["Island", "Mox Sapphire", "Mox Sapphire", "Mox Sapphire", "Eluge, the Shoreless Sea", "Probe Mute"], &[]],
        vec![
            Step::Play(0, 1, "Island"),
            Step::Cast(0, 1, "Mox Sapphire", None),
            Step::Cast(0, 1, "Mox Sapphire", None),
            Step::Cast(0, 1, "Mox Sapphire", None),
            Step::Cast(0, 1, "Eluge, the Shoreless Sea", None),
            Step::Cast(0, 1, "Probe Mute", Some("Eluge, the Shoreless Sea")),
        ],
        1,
        |_, _| {},
    );
    let eluge = *game.state.battlefield().iter().find(|&&id| game.name(id) == "Eluge, the Shoreless Sea").expect("Eluge");
    let c = game.chars(eluge);
    assert_eq!((c.power, c.toughness), (Some(0), Some(2)));
}

/// Eluge with 300 flooded lands: its reduction is 300 {U}, which a cost's {U} absorbs and the rest takes generic mana
/// (CR 118.7c), so Mind Spring at X = 254 ({254}{U}{U}) is free; and Expansion's {U/R}{U/R}, announced as blue (CR 601.2b),
/// is free too. Bot1 floods every land on turn 1; the totals are the table's on turn 3.
#[test]
fn eluges_reduction_past_255_flooded_lands_takes_generic_mana_and_hybrid_symbols() {
    let mut totals: Option<(u32, u32)> = None;
    let names = |list: &[&str]| -> Vec<CardId> { list.iter().map(|n| table().find(n).unwrap_or_else(|| panic!("no card {n}"))).collect() };
    let mut deck = names(&["Probe Flood All", "Probe Spring", "Probe Expansion"]);
    deck.extend(names(&["Island"; 27]));
    let mut game = Game::new(table(), &Setup { decks: vec![deck, names(&["Island"; 30])], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    for name in std::iter::once("Eluge, the Shoreless Sea").chain(std::iter::repeat_n("Island", 300)) {
        let id = game.state.push_obj(Obj::new(table().find(name).unwrap(), PlayerId(0), Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
    }
    for _ in 0..3000 {
        game.run();
        let request = game.pending().cloned().expect("a question");
        let mine = request.player == PlayerId(0) && game.state.stack.is_empty() && game.state.turn.step() == cantrip_engine::Step::PrecombatMain;
        if mine && game.state.turn.number == 3 && matches!(request.kind, RequestKind::Priority { .. }) {
            let total = |name: &str| {
                let card = *game.state.zone(Zone::hand(PlayerId(0))).iter().find(|&&c| game.name(c) == name).expect("in hand");
                cast_total(&game, PlayerId(0), &ActionOption::on(ActionKind::CastSpell, card, 0)).expect("a total").1.mana_value()
            };
            totals = Some((total("Probe Spring"), total("Probe Expansion")));
            break;
        }
        let answer = match &request.kind {
            RequestKind::Priority { options } if mine && game.state.turn.number == 1 => options
                .iter()
                .find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|c| game.name(c) == "Probe Flood All"))
                .map_or(Answer::Pass, |o| Answer::Act(Action::of(o))),
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::PayCosts { options, .. } => Answer::Act(Action::of(
                options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).or(options.first()).expect("a way to pay"),
            )),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().rev().copied().take(*count as usize).collect()),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert_eq!(totals, Some((0, 0)), "Mind Spring at X = 254 and Expansion, each the first instant of the turn under 300 flooded lands");
}

/// Eluge with one flooded land (a {U} reduction) and three Moxen; on turn 3 Bot1 casts `card`, answering the announcement of
/// its hybrid symbols (CR 601.2b) with `choose`, and paying with the Moxen it is asked for. The announcement's options, and
/// the Moxen tapped.
fn announced(card: &str, choose: Vec<u8>) -> (Vec<String>, Vec<String>) {
    let names = |list: &[&str]| -> Vec<CardId> { list.iter().map(|n| table().find(n).unwrap_or_else(|| panic!("no card {n}"))).collect() };
    let mut deck = names(&["Probe Flood All", card]);
    deck.extend(names(&["Swamp"; 28]));
    let mut game = Game::new(table(), &Setup { decks: vec![deck, names(&["Island"; 30])], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    for name in ["Eluge, the Shoreless Sea", "Island", "Mox Ruby", "Mox Jet", "Mox Sapphire"] {
        let id = game.state.push_obj(Obj::new(table().find(name).unwrap(), PlayerId(0), Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
    }
    let mut options = Vec::new();
    let mut cast = false;
    for _ in 0..3000 {
        game.run();
        let request = game.pending().cloned().expect("a question");
        let mine = request.player == PlayerId(0) && game.state.stack.is_empty() && game.state.turn.step() == cantrip_engine::Step::PrecombatMain;
        if cast && game.state.stack.is_empty() {
            break;
        }
        let answer = match &request.kind {
            RequestKind::Priority { options } if mine && (game.state.turn.number == 1 || game.state.turn.number == 3) => {
                let wanted = if game.state.turn.number == 1 { "Probe Flood All" } else { card };
                match options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|c| game.name(c) == wanted)) {
                    Some(o) => {
                        cast |= game.state.turn.number == 3;
                        Answer::Act(Action::of(o))
                    }
                    None => Answer::Pass,
                }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseModes { modes, additional_cost: true, .. } => {
                options = modes.iter().map(|m| cantrip_engine::label_text(m).to_string()).collect();
                Answer::Modes(choose.clone())
            }
            // The Mox of the color still owed.
            RequestKind::PayCosts { options, remaining, .. } => {
                let mox = [(Mana::U, "Mox Sapphire"), (Mana::B, "Mox Jet"), (Mana::R, "Mox Ruby")]
                    .into_iter()
                    .find(|&(m, _)| remaining.symbols[m as usize] > 0)
                    .map(|(_, name)| name);
                Answer::Act(Action::of(
                    options
                        .iter()
                        .find(|o| o.kind == ActionKind::ConfirmCasting)
                        .or_else(|| options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| mox.is_some_and(|name| game.name(s) == name))))
                        .expect("a way to pay"),
                ))
            }
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().rev().copied().take(*count as usize).collect()),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert!(cast, "{card} was cast");
    let tapped = game.state.battlefield().iter().filter(|&&id| game.state.obj(id).tapped && game.name(id).to_string().starts_with("Mox")).map(|&id| game.name(id).to_string()).collect();
    (options, tapped)
}

/// Unnerving Assault's {U/R} under a {U} reduction: announced as blue, the reduction takes it and nothing is paid; announced
/// as red on purpose - to spend red mana on it - the reduction takes generic mana it has none of, and {R} is paid.
#[test]
fn a_hybrid_symbol_a_reduction_meets_is_announced_and_may_be_paid_with_its_other_half() {
    let (options, tapped) = announced("Probe Assault", vec![]);
    assert_eq!(options, ["Pay {U/R} with {R}"]);
    assert!(tapped.is_empty(), "announced as blue, it is reduced to nothing: {tapped:?}");
    let (_, tapped) = announced("Probe Assault", vec![0]);
    assert_eq!(tapped, ["Mox Ruby"]);
}

/// {U/R}{U/B} under one {U}: announced {U}{U}, one is taken off and {U} paid; with {U/B} announced as black, {U} comes off
/// the {U/R} and {B} is paid.
#[test]
fn hybrid_symbols_of_different_pairs_are_each_announced() {
    let (options, tapped) = announced("Probe Mixed", vec![]);
    assert_eq!(options, ["Pay {U/R} with {R}", "Pay {U/B} with {B}"]);
    assert_eq!(tapped, ["Mox Sapphire"]);
    let (_, tapped) = announced("Probe Mixed", vec![1]);
    assert_eq!(tapped, ["Mox Jet"]);
}
