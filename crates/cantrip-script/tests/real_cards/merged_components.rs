//! Cards that find "that card" or "it" in the zone a permanent went to, played with a merged permanent (CR 730, 702.140): one
//! permanent leaves, and an effect that can find the new object it becomes finds every card it was made of, the same done to
//! each (CR 730.3c). No printed mutate card is in the engine yet, so a test card mutates onto a creature or token, over or
//! under it, beside the printed card under test, and the permanent is destroyed, exiled or put into a library the way the
//! card needs. Every test is played through the engine's questions, as a player would.

use std::cell::RefCell;
use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_mechanics::tokens;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn creature_on_battlefield(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

fn creatures(g: &dyn Query) -> Vec<ObjectId> {
    g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Creature)).collect()
}

thread_local! {
    /// The names Probe Portal leaves in the hand, for a test that casts them.
    static HELD: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

/// "Put each permanent card in your hand onto the battlefield, but for Probe Mutant and the cards the test holds back."
fn portal(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let hand = g.state().zone(Zone::hand(item.controller)).to_vec();
    let entering: Vec<(ObjectId, PlayerId)> = hand
        .into_iter()
        .filter(|&c| {
            let name = g.chars(c).name;
            name != "Probe Mutant" && !HELD.with(|h| h.borrow().contains(&name)) && cantrip_mechanics::targets::is_permanent_card(&g.chars(c))
        })
        .map(|c| (c, item.controller))
        .collect();
    g.put_all_onto_battlefield(&entering)?;
    Ok(())
}

/// "Add {W}{W}{W}{B}{B}{B}{G}{G}{G}{U}{U}{U}{U}{U}." The pool lasts the step, which a test spends in its first main phase.
fn ritual(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    for (mana, n) in [(Mana::W, 3), (Mana::B, 3), (Mana::G, 3), (Mana::U, 5)] {
        g.add_mana(item.controller, mana, n);
    }
    Ok(())
}

/// "Destroy target creature."
fn smite(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(id) = item.target(0) {
        g.destroy(id);
    }
    Ok(())
}

fn nothing(_: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// "Every creature becomes an Elf, losing its other creature types."
fn mold(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let all = creatures(g);
    instruction(g, |g| {
        for id in all {
            g.add_effect(item.object, id, EffectKind::LoseAllCreatureTypes, Duration::Indefinite);
            g.add_effect(item.object, id, EffectKind::AddSubtype(Subtype::Elf), Duration::Indefinite);
        }
    });
    Ok(())
}

/// "Put a -1/-1 counter on each creature."
fn shrink(g: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    let counters: Vec<(ObjectId, CounterKind, i32)> = creatures(g).into_iter().map(|c| (c, CounterKind::MinusOne, 1)).collect();
    g.add_counters_together(&counters);
    Ok(())
}

/// "Mill seven cards."
fn mill(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.mill(item.controller, 7);
    Ok(())
}

/// "Create a 1/1 Spirit creature token."
fn make_spirit(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::SPIRIT, 1)?;
    Ok(())
}

static PORTAL: CardDef = CardDef { name: "Probe Portal", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], portal), ..CardDef::BLANK };
static RITUAL: CardDef = CardDef { name: "Probe Ritual", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], ritual), ..CardDef::BLANK };
static SMITE: CardDef = CardDef { name: "Probe Smite", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[target(creature_on_battlefield)], smite), ..CardDef::BLANK };
static NUDGE: CardDef = CardDef { name: "Probe Nudge", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[target(creature_on_battlefield)], nothing), ..CardDef::BLANK };
static MOLD: CardDef = CardDef { name: "Probe Mold", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], mold), ..CardDef::BLANK };
static SHRINK: CardDef = CardDef { name: "Probe Shrink", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], shrink), ..CardDef::BLANK };
static MILL: CardDef = CardDef { name: "Probe Mill", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], mill), ..CardDef::BLANK };
static MAKER: CardDef = CardDef { name: "Probe Maker", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], make_spirit), ..CardDef::BLANK };

/// "Mutate {0}": a 1/1 Elf Beast artifact creature that costs nothing.
static MUTANT: CardDef = CardDef {
    name: "Probe Mutant",
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Elf, Subtype::Beast]),
    power: Some(1),
    toughness: Some(1),
    ways: &[WayDef { kind: WayKind::Mutate, ..way(Cost::mana(ManaCost::ZERO), ZoneKind::Hand, "Mutate {0}") }],
    text: "Mutate {0}",
    ..CardDef::BLANK
};

/// A 2/2 Bear that costs nothing.
static BEAR: CardDef = CardDef {
    name: "Probe Bear",
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Bear]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// A planeswalker with 3 loyalty that costs nothing, for an opponent to control.
static WALKER: CardDef = CardDef { name: "Probe Walker", types: TypeSet::of(&[CardType::Planeswalker]), loyalty: Some(3), ..CardDef::BLANK };

/// "Put each Probe Walker in your hand onto the battlefield under an opponent's control."
fn gift(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let walkers: Vec<(ObjectId, PlayerId)> = g.state().zone(Zone::hand(item.controller)).iter().copied().filter(|&c| g.chars(c).name == "Probe Walker").map(|c| (c, THEM)).collect();
    g.put_all_onto_battlefield(&walkers)?;
    Ok(())
}

static GIFT: CardDef = CardDef { name: "Probe Gift", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], gift), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        CardTable::new(cantrip_cards::all().into_iter().chain([&PORTAL, &RITUAL, &SMITE, &NUDGE, &MOLD, &SHRINK, &MILL, &MAKER, &MUTANT, &BEAR, &WALKER, &GIFT]))
    })
}

/// A cast to make at sorcery speed, when the stack is empty: `name`, for its mutate cost if `mutate`, at the object named
/// `target` if it has a target.
#[derive(Clone, Copy)]
struct Cast {
    name: &'static str,
    mutate: bool,
    target: Option<&'static str>,
}

const fn cast(name: &'static str) -> Cast {
    Cast { name, mutate: false, target: None }
}

impl Cast {
    const fn mutating(self, target: &'static str) -> Cast {
        Cast { mutate: true, target: Some(target), ..self }
    }
    const fn at(self, target: &'static str) -> Cast {
        Cast { target: Some(target), ..self }
    }
}

/// How the questions that are not a test's own are answered.
#[derive(Clone, Copy)]
struct Cfg {
    /// Whether a mutating spell goes on top of the permanent it merges with (CR 702.140c).
    on_top: bool,
    /// The name of the object a triggered ability's target is, where the first candidate would not do.
    pick: Option<&'static str>,
}

const OVER: Cfg = Cfg { on_top: true, pick: None };
const UNDER: Cfg = Cfg { on_top: false, pick: None };

impl Cfg {
    const fn picking(self, name: &'static str) -> Cfg {
        Cfg { pick: Some(name), ..self }
    }
}

/// A game with `hand` as Bot1's opening hand (and the rest of its library Islands), at the first main phase of turn 1.
fn start(hand: &[&str], held: &[&'static str]) -> Game {
    let names = |list: &[&str]| -> Vec<CardId> { list.iter().map(|n| table().find(n).unwrap_or_else(|| panic!("no card {n}"))).collect() };
    let mut first = names(hand);
    first.extend(names(&vec!["Island"; 40 - hand.len()]));
    let mut second = names(&["Island"; 40]);
    second.truncate(40);
    HELD.with(|h| *h.borrow_mut() = held.to_vec());
    let mut game = Game::new(table(), &Setup { decks: vec![first, second], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    game
}

fn pick(game: &Game, candidates: &[Target], want: Option<&str>) -> Target {
    want.and_then(|w| candidates.iter().copied().find(|t| t.object().is_some_and(|id| game.name(id) == w))).unwrap_or(candidates[0])
}

/// Plays `steps` one after another, each when Bot1 has priority with an empty stack, answering every other question as `cfg`
/// and `hook` say (the hook first), and going on until the steps are done and `done` holds.
fn run(game: &mut Game, steps: &[Cast], cfg: Cfg, hook: &mut dyn FnMut(&Game, &Request) -> Option<Answer>, done: &dyn Fn(&Game) -> bool) {
    let mut steps = steps.iter().peekable();
    for _ in 0..4000 {
        game.run();
        if steps.peek().is_none() && done(game) {
            return;
        }
        let request = game.pending().cloned().expect("the game goes on");
        let answer = match hook(game, &request) {
            Some(answer) => answer,
            None => match &request.kind {
                RequestKind::Priority { options } if request.player == YOU && game.state.stack.is_empty() && steps.peek().is_some() => {
                    let step = *steps.peek().unwrap();
                    let found = options
                        .iter()
                        .find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == step.name) && (o.index == 1) == step.mutate);
                    let Some(option) = found else {
                        // Not yet: a sorcery waits for a main phase. In one, it is a mistake of the test.
                        assert!(game.state.turn.step() != Step::PrecombatMain, "{} is not offered (mutate: {}); options: {options:?}", step.name, step.mutate);
                        game.answer(request.id, request.player, Answer::Pass).expect("passing");
                        continue;
                    };
                    steps.next();
                    let mut action = Action::of(option);
                    if option.targets_required > 0 {
                        action.targets = vec![pick(game, &option.target_candidates, step.target)];
                    }
                    Answer::Act(action)
                }
                RequestKind::Priority { .. } => Answer::Pass,
                RequestKind::PayCosts { options, .. } => {
                    let pay = options
                        .iter()
                        .find(|o| o.kind == ActionKind::ConfirmCasting)
                        .or_else(|| options.iter().find(|o| o.kind == ActionKind::AssignMana))
                        .or_else(|| options.iter().find(|o| o.kind == ActionKind::ActivateAbility))
                        .expect("something pays");
                    Answer::Act(Action::of(pay))
                }
                RequestKind::UnlessPays { options, .. } => match options.iter().find(|o| o.kind == ActionKind::PayCost) {
                    Some(pay) => Answer::Act(Action::of(pay)),
                    None => Answer::Act(Action::of(options.iter().find(|o| o.kind == ActionKind::DeclineCost).expect("declining is offered"))),
                },
                RequestKind::May { .. } => Answer::Yes,
                RequestKind::ChooseTarget { candidates, .. } => Answer::Targets(vec![pick(game, candidates, cfg.pick)]),
                RequestKind::ChooseModes { modes, eligible, at_most, count, .. } => {
                    if modes.first() == Some(&Label::Mutate { on_top: true }) {
                        Answer::Modes(vec![if cfg.on_top { 0 } else { 1 }])
                    } else {
                        Answer::Modes(eligible.iter().copied().take(usize::from(*at_most).max(usize::from(*count))).collect())
                    }
                }
                RequestKind::ChooseCards { cards, rule, .. } => {
                    let eligible = rule.among.clone().unwrap_or_else(|| cards.clone());
                    Answer::Cards(eligible.into_iter().take(1).collect())
                }
                RequestKind::OrderTriggers { sources, .. } => Answer::Order((0..sources.len() as u32).collect()),
                RequestKind::DistributeCards { cards, destinations, .. } => {
                    let mut placements = vec![cards.clone()];
                    placements.resize(destinations.len().max(1), Vec::new());
                    Answer::Placements { placements, shuffle: false }
                }
                RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
                RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
                RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
                other => panic!("unexpected question {other:?}"),
            },
        };
        game.answer(request.id, request.player, answer).unwrap_or_else(|e| panic!("{e:?} answering {:?}", request.kind));
    }
    panic!("the game did not settle");
}

/// Plays `steps` to the point where nothing is on the stack or waiting to go on it.
fn settle(game: &mut Game, steps: &[Cast], cfg: Cfg, hook: &mut dyn FnMut(&Game, &Request) -> Option<Answer>) {
    run(game, steps, cfg, hook, &|g| g.state.stack.is_empty() && g.state.triggers.is_empty() && matches!(g.pending().map(|r| &r.kind), Some(RequestKind::Priority { .. })));
}

/// Plays on until turn `turn` begins, with every question answered as `cfg` says.
fn until_turn(game: &mut Game, cfg: Cfg, turn: u32, hook: &mut dyn FnMut(&Game, &Request) -> Option<Answer>) {
    run(game, &[], cfg, hook, &|g| g.state.turn.number >= turn);
}

fn no_hook() -> impl FnMut(&Game, &Request) -> Option<Answer> {
    |_, _| None
}

/// The names of the objects in `zone`, sorted.
fn names(game: &Game, zone: Zone) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = game.state.zone(zone).iter().map(|&id| game.name(id)).collect();
    names.sort_unstable();
    names
}

/// How many objects in Bot1's graveyard have one of these names.
fn in_graveyard(game: &Game, names: &[&str]) -> usize {
    names.iter().map(|n| count(game, Zone::graveyard(YOU), n)).sum()
}

/// How many objects in `zone` have this name.
fn count(game: &Game, zone: Zone, name: &str) -> usize {
    game.state.zone(zone).iter().filter(|&&id| game.name(id) == name).count()
}

/// The objects on the battlefield with this name.
fn on_battlefield(game: &Game, name: &str) -> Vec<ObjectId> {
    game.state.battlefield().iter().copied().filter(|&id| game.name(id) == name).collect()
}

fn counters(game: &Game, id: ObjectId, kind: CounterKind) -> i32 {
    game.state.obj(id).counters(kind)
}

/// Bot1's hand `hand` and the name of the merged permanent: the test card on top if `cfg.on_top`, else the printed card.
fn top<'a>(cfg: Cfg, printed: &'a str) -> &'a str {
    if cfg.on_top { "Probe Mutant" } else { printed }
}

/// The steps that put `printed` and the Mutant on the battlefield, merged.
fn merged_steps(printed: &'static str) -> [Cast; 2] {
    [cast("Probe Portal"), cast("Probe Mutant").mutating(printed)]
}

/// Each card returns together, tapped, with two stun counters (CR 730.3c, 122.1d): Unstoppable Slasher.
#[test]
fn unstoppable_slasher_returns_every_card_of_a_merged_permanent() {
    for cfg in [OVER, UNDER] {
        let mut game = start(&["Probe Portal", "Unstoppable Slasher", "Probe Mutant", "Probe Smite"], &[]);
        let mut steps = merged_steps("Unstoppable Slasher").to_vec();
        steps.push(cast("Probe Smite").at(top(cfg, "Unstoppable Slasher")));
        settle(&mut game, &steps, cfg, &mut no_hook());
        assert_eq!(in_graveyard(&game, &["Unstoppable Slasher", "Probe Mutant"]), 0, "both cards left the graveyard");
        let back: Vec<ObjectId> = game.state.battlefield().iter().copied().filter(|&id| creatures(&game).contains(&id)).collect();
        assert_eq!(back.len(), 2, "both cards returned");
        for id in back {
            assert!(game.state.obj(id).tapped, "{} returned tapped", game.name(id));
            assert_eq!(counters(&game, id, CounterKind::Stun), 2, "{} has two stun counters", game.name(id));
        }
    }
}

/// Each card returns with a +1/+1 counter, loses all abilities and has haste (CR 730.3c): Hellcat (a Human, which must stop being
/// one to be mutated onto, CR 702.140a).
#[test]
fn hellcat_returns_every_card_of_a_merged_permanent_changed() {
    let mut game = start(&["Probe Portal", "Hellcat, Undying Vigilante", "Probe Mold", "Probe Mutant", "Probe Smite"], &[]);
    let steps = [cast("Probe Portal"), cast("Probe Mold"), cast("Probe Mutant").mutating("Hellcat, Undying Vigilante"), cast("Probe Smite").at("Probe Mutant")];
    settle(&mut game, &steps, OVER, &mut no_hook());
    let back = creatures(&game);
    assert_eq!(back.len(), 2, "both cards returned");
    for id in back {
        assert_eq!(counters(&game, id, CounterKind::PlusOne), 1, "{} has its counter", game.name(id));
        let chars = game.chars(id);
        assert!(chars.keywords.has(Keyword::Haste), "{} has haste", game.name(id));
        assert!(chars.abilities.iter().all(|a| a.is_none()), "{} lost its abilities", game.name(id));
    }
}

/// Each card returns and loses all abilities (CR 730.3c): Retched Wretch, which has a -1/-1 counter.
#[test]
fn retched_wretch_returns_every_card_of_a_merged_permanent_without_abilities() {
    let mut game = start(&["Probe Portal", "Retched Wretch", "Probe Mutant", "Probe Shrink", "Probe Smite"], &[]);
    let steps = [cast("Probe Portal"), cast("Probe Mutant").mutating("Retched Wretch"), cast("Probe Shrink"), cast("Probe Smite").at("Retched Wretch")];
    // The Wretch is under the Mutant here (CR 702.140c): its own values are the topmost.
    settle(&mut game, &steps, UNDER, &mut no_hook());
    let back = creatures(&game);
    assert_eq!(back.len(), 2, "both cards returned");
    for id in back {
        assert!(game.chars(id).abilities.iter().all(|a| a.is_none()), "{} lost its abilities", game.name(id));
    }
}

/// Each card returns as an artifact that is no creature (CR 730.3c, 614.12): Tom, Bert, and William.
#[test]
fn tom_bert_and_william_return_every_card_of_a_merged_permanent_as_artifacts() {
    let mut game = start(&["Probe Portal", "Tom, Bert, and William", "Probe Mutant", "Probe Smite"], &[]);
    let steps = [cast("Probe Portal"), cast("Probe Mutant").mutating("Tom, Bert, and William"), cast("Probe Smite").at("Probe Mutant")];
    settle(&mut game, &steps, OVER, &mut no_hook());
    assert!(creatures(&game).is_empty(), "none of them is a creature");
    let artifacts: Vec<ObjectId> = game.state.battlefield().iter().copied().filter(|&id| game.chars(id).is(CardType::Artifact)).collect();
    assert_eq!(artifacts.len(), 2, "both cards returned as artifacts: {:?}", names(&game, Zone::BATTLEFIELD));
}

/// Each card returns as an Aura enchanting a Forest (CR 730.3c, 614.12): Old-Growth Troll.
#[test]
fn old_growth_troll_returns_every_card_of_a_merged_permanent_as_auras() {
    let mut game = start(&["Probe Portal", "Old-Growth Troll", "Probe Mutant", "Probe Smite", "Forest", "Forest"], &[]);
    let steps = [cast("Probe Portal"), cast("Probe Mutant").mutating("Old-Growth Troll"), cast("Probe Smite").at("Probe Mutant")];
    settle(&mut game, &steps, OVER, &mut no_hook());
    let auras: Vec<ObjectId> = game.state.battlefield().iter().copied().filter(|&id| game.chars(id).has_subtype(Subtype::Aura)).collect();
    assert_eq!(auras.len(), 2, "both cards returned as Auras: {:?}", names(&game, Zone::BATTLEFIELD));
    assert!(creatures(&game).is_empty());
}

/// Every card is exiled, and the Treasures are made once (CR 730.3c): Ancient Adamantoise.
#[test]
fn ancient_adamantoise_exiles_every_card_of_a_merged_permanent() {
    let mut game = start(&["Probe Portal", "Ancient Adamantoise", "Probe Mutant", "Probe Smite"], &[]);
    let steps = [cast("Probe Portal"), cast("Probe Mutant").mutating("Ancient Adamantoise"), cast("Probe Smite").at("Probe Mutant")];
    settle(&mut game, &steps, OVER, &mut no_hook());
    assert_eq!(names(&game, Zone::exile(YOU)), vec!["Ancient Adamantoise", "Probe Mutant"]);
    assert_eq!(in_graveyard(&game, &["Ancient Adamantoise", "Probe Mutant"]), 0);
    let treasures = game.state.battlefield().iter().filter(|&&id| game.state.obj(id).token).count();
    assert_eq!(treasures, 10, "ten Treasures, once: {:?}", names(&game, Zone::BATTLEFIELD));
}

/// Every card may be exiled together, and the ability's target is not any of them (CR 730.3c): Cavalier of Thorns puts another
/// card from the graveyard on top of the library.
#[test]
fn cavalier_of_thorns_exiles_every_card_of_a_merged_permanent_and_targets_neither() {
    let mut game = start(&["Probe Portal", "Cavalier of Thorns", "Probe Mutant", "Probe Smite"], &[]);
    let steps = [cast("Probe Portal"), cast("Probe Mutant").mutating("Cavalier of Thorns"), cast("Probe Smite").at("Probe Mutant")];
    let mut candidates_seen = Vec::new();
    let mut hook = |g: &Game, r: &Request| {
        if let RequestKind::ChooseTarget { candidates, .. } = &r.kind
            && g.state.zone(Zone::graveyard(YOU)).iter().any(|&id| g.name(id) == "Cavalier of Thorns")
        {
            candidates_seen = candidates.iter().filter_map(|t| t.object()).map(|id| g.name(id)).collect();
        }
        None
    };
    settle(&mut game, &steps, OVER, &mut hook);
    assert!(!candidates_seen.is_empty(), "the ability had targets");
    assert!(candidates_seen.iter().all(|&n| n != "Cavalier of Thorns" && n != "Probe Mutant"), "neither card of the permanent is a target: {candidates_seen:?}");
    assert!(candidates_seen.contains(&"Island"), "the other cards are: {candidates_seen:?}");
    assert_eq!(names(&game, Zone::exile(YOU)), vec!["Cavalier of Thorns", "Probe Mutant"]);
    assert_eq!(game.name(game.state.zone(Zone::library(YOU))[0]), candidates_seen[0], "the target is on top of the library");
}

/// Every card is exiled together, and only then does the reflexive ability trigger (CR 730.3c): Paramecia Coloniex.
#[test]
fn paramecia_coloniex_exiles_every_card_of_a_merged_permanent() {
    let mut game = start(&["Probe Portal", "Paramecia Coloniex", "Probe Mutant", "Probe Smite"], &[]);
    let steps = [cast("Probe Portal"), cast("Probe Mutant").mutating("Paramecia Coloniex"), cast("Probe Smite").at("Probe Mutant")];
    settle(&mut game, &steps, OVER, &mut no_hook());
    assert_eq!(names(&game, Zone::exile(YOU)), vec!["Paramecia Coloniex", "Probe Mutant"]);
}

/// Each card is returned at the next end step with a +1/+1 counter (CR 730.3c, 603.7c): Dr. McCoy.
#[test]
fn dr_mccoy_returns_every_card_of_a_merged_creature_that_dies() {
    let mccoy = "Dr. Leonard \"Bones\" McCoy";
    let mut game = start(&["Probe Portal", "Probe Bear", "Probe Mutant", "Probe Ritual", mccoy, "Probe Smite"], &[mccoy]);
    let steps = [cast("Probe Portal"), cast("Probe Mutant").mutating("Probe Bear"), cast("Probe Ritual"), cast(mccoy), cast("Probe Smite").at("Probe Mutant")];
    let mut hook = |_: &Game, r: &Request| match &r.kind {
        // The second mode: "choose up to one other target creature. When it dies this turn, ..."
        RequestKind::ChooseModes { modes, .. } if modes.first() != Some(&Label::Mutate { on_top: true }) => Some(Answer::Modes(vec![1])),
        _ => None,
    };
    settle(&mut game, &steps, OVER.picking("Probe Mutant"), &mut hook);
    assert_eq!(in_graveyard(&game, &["Probe Bear", "Probe Mutant"]), 2, "both died together");
    until_turn(&mut game, OVER.picking("Probe Mutant"), 2, &mut no_hook());
    for name in ["Probe Bear", "Probe Mutant"] {
        let back = on_battlefield(&game, name);
        assert_eq!(back.len(), 1, "{name} returned at the end step");
        assert_eq!(counters(&game, back[0], CounterKind::PlusOne), 1, "{name} has its counter");
    }
}

/// Each card is returned at the next end step under its controller's control (CR 730.3c, 603.7c): Avacyn, Angel of Horror.
#[test]
fn avacyn_returns_every_card_of_a_merged_creature_that_dies() {
    let mut game = start(&["Probe Portal", "Avacyn, Angel of Horror", "Probe Bear", "Probe Mutant", "Probe Smite"], &[]);
    let steps = [cast("Probe Portal"), cast("Probe Mutant").mutating("Probe Bear"), cast("Probe Smite").at("Probe Mutant")];
    settle(&mut game, &steps, OVER, &mut no_hook());
    assert_eq!(in_graveyard(&game, &["Probe Bear", "Probe Mutant"]), 2);
    until_turn(&mut game, OVER, 2, &mut no_hook());
    assert_eq!(on_battlefield(&game, "Probe Bear").len(), 1, "the Bear returned");
    assert_eq!(on_battlefield(&game, "Probe Mutant").len(), 1, "the Mutant returned");
    assert_eq!(in_graveyard(&game, &["Probe Bear", "Probe Mutant"]), 0);
}

/// "Return that card to your hand" returns every card of the artifact, which died once (CR 730.3, 730.3c): Pia's Revolution, its
/// opponent declining the damage.
#[test]
fn pias_revolution_returns_every_card_of_a_merged_artifact() {
    let mut game = start(&["Probe Portal", "Pia's Revolution", "Probe Bear", "Probe Mutant", "Probe Smite"], &[]);
    let steps = [cast("Probe Portal"), cast("Probe Mutant").mutating("Probe Bear"), cast("Probe Smite").at("Probe Mutant")];
    let mut asked = 0;
    let mut hook = |_: &Game, r: &Request| match &r.kind {
        RequestKind::May { .. } if r.player == THEM => {
            asked += 1;
            Some(Answer::No)
        }
        _ => None,
    };
    settle(&mut game, &steps, OVER, &mut hook);
    assert_eq!(asked, 1, "the artifact died once: one ability, one choice");
    assert_eq!(count(&game, Zone::hand(YOU), "Probe Bear"), 1, "the Bear is back in hand");
    assert_eq!(count(&game, Zone::hand(YOU), "Probe Mutant"), 1, "the Mutant is back in hand");
    assert_eq!(in_graveyard(&game, &["Probe Bear", "Probe Mutant"]), 0);
}

/// Every card is returned to its owner's hand when the payment is made (CR 730.3c): Decaying Soil, with threshold.
#[test]
fn decaying_soil_returns_every_card_of_a_merged_creature() {
    let mut game = start(&["Probe Portal", "Decaying Soil", "Probe Bear", "Probe Mutant", "Probe Mill", "Probe Smite", "Probe Ritual"], &[]);
    let steps = [
        cast("Probe Portal"),
        cast("Probe Mill"),
        cast("Probe Ritual"),
        cast("Probe Mutant").mutating("Probe Bear"),
        cast("Probe Smite").at("Probe Mutant"),
    ];
    settle(&mut game, &steps, OVER, &mut no_hook());
    assert_eq!(count(&game, Zone::hand(YOU), "Probe Bear"), 1, "the Bear is back in hand");
    assert_eq!(count(&game, Zone::hand(YOU), "Probe Mutant"), 1, "the Mutant is back in hand");
    assert_eq!(count(&game, Zone::graveyard(YOU), "Probe Bear") + count(&game, Zone::graveyard(YOU), "Probe Mutant"), 0);
}

/// A merged permanent with a token on top is a token, but its cards are put into a library as cards (CR 730.2d, 730.3e): Wan
/// Shi Tong makes Spirits for them. Its own enters ability puts the merged Spirit into its owner's library.
#[test]
fn wan_shi_tong_sees_the_cards_of_a_merged_token_go_into_a_library() {
    let mut game = start(&["Probe Maker", "Probe Mutant", "Wan Shi Tong, All-Knowing", "Probe Ritual"], &["Wan Shi Tong, All-Knowing"]);
    let steps = [cast("Probe Maker"), cast("Probe Mutant").mutating("Spirit Token"), cast("Probe Ritual"), cast("Wan Shi Tong, All-Knowing")];
    settle(&mut game, &steps, UNDER.picking("Spirit Token"), &mut no_hook());
    assert_eq!(count(&game, Zone::library(YOU), "Probe Mutant"), 1, "the Mutant card is in the library");
    assert_eq!(on_battlefield(&game, "Spirit Token").len(), 2, "two Spirits for the cards put into a library, the merged token itself gone");
}

/// "If you descended this turn": a card component of a merged token was put into the graveyard (CR 730.3e, 700.11): Corpses of
/// the Lost returns itself at the end step.
#[test]
fn corpses_of_the_lost_counts_the_cards_of_a_merged_token_that_died() {
    let mut game = start(&["Probe Portal", "Corpses of the Lost", "Probe Maker", "Probe Mutant", "Probe Smite"], &[]);
    let steps = [cast("Probe Portal"), cast("Probe Maker"), cast("Probe Mutant").mutating("Spirit Token"), cast("Probe Smite").at("Spirit Token")];
    settle(&mut game, &steps, UNDER, &mut no_hook());
    assert_eq!(count(&game, Zone::graveyard(YOU), "Probe Mutant"), 1, "the Mutant card is in the graveyard");
    until_turn(&mut game, UNDER, 2, &mut no_hook());
    assert_eq!(count(&game, Zone::hand(YOU), "Corpses of the Lost"), 1, "descended: it returned to hand");
}

/// Each card returns to the battlefield with a +1/+1 counter (CR 730.3c, 400.7): Daydream.
#[test]
fn daydream_returns_every_card_of_a_merged_creature() {
    let mut game = start(&["Probe Portal", "Probe Bear", "Probe Mutant", "Probe Ritual", "Daydream"], &[]);
    let steps = [cast("Probe Portal"), cast("Probe Mutant").mutating("Probe Bear"), cast("Probe Ritual"), cast("Daydream").at("Probe Mutant")];
    settle(&mut game, &steps, OVER, &mut no_hook());
    let back = creatures(&game);
    assert_eq!(back.len(), 2, "both cards returned: {:?}", names(&game, Zone::BATTLEFIELD));
    for id in back {
        assert_eq!(counters(&game, id, CounterKind::PlusOne), 1, "{} has its counter", game.name(id));
    }
    assert!(game.state.zone(Zone::exile(YOU)).is_empty());
}

/// Each card returns tapped under your control and is exiled at the next end step (CR 730.3c): Vindictive Triumph.
#[test]
fn vindictive_triumph_returns_and_exiles_every_card_of_a_merged_creature() {
    let mut game = start(&["Probe Portal", "Probe Bear", "Probe Mutant", "Probe Ritual", "Vindictive Triumph"], &[]);
    let steps = [cast("Probe Portal"), cast("Probe Mutant").mutating("Probe Bear"), cast("Probe Ritual"), cast("Vindictive Triumph").at("Probe Mutant")];
    settle(&mut game, &steps, OVER, &mut no_hook());
    let back = creatures(&game);
    assert_eq!(back.len(), 2, "both cards returned: {:?}", names(&game, Zone::BATTLEFIELD));
    assert!(back.iter().all(|&id| game.state.obj(id).tapped), "tapped");
    until_turn(&mut game, OVER, 2, &mut no_hook());
    assert!(creatures(&game).is_empty(), "both exiled at the end step");
    assert_eq!(names(&game, Zone::exile(YOU)), vec!["Probe Bear", "Probe Mutant"]);
}

/// Each card returns tapped, and dies with the creature that Pain 101 gave "return it" (CR 730.3c): Pain 101.
#[test]
fn pain_101_returns_every_card_of_a_merged_creature() {
    let mut game = start(&["Probe Portal", "Probe Bear", "Probe Mutant", "Probe Ritual", "Pain 101", "Probe Smite"], &[]);
    let steps = [
        cast("Probe Portal"),
        cast("Probe Mutant").mutating("Probe Bear"),
        cast("Probe Ritual"),
        cast("Pain 101").at("Probe Mutant"),
        cast("Probe Smite").at("Probe Mutant"),
    ];
    settle(&mut game, &steps, OVER, &mut no_hook());
    let back = creatures(&game);
    assert_eq!(back.len(), 2, "both cards returned: {:?}", names(&game, Zone::BATTLEFIELD));
    assert!(back.iter().all(|&id| game.state.obj(id).tapped), "tapped");
}

/// Each card is exiled and, unless {3}{B} is paid, returned at the next end step (CR 730.3c): Koya, Death from Above.
#[test]
fn koya_returns_every_card_of_a_merged_creature_unless_paid_for() {
    let koya = "Koya, Death from Above";
    let mut game = start(&["Probe Portal", "Probe Bear", "Probe Mutant", "Probe Ritual", koya], &[koya]);
    let steps = [cast("Probe Portal"), cast("Probe Mutant").mutating("Probe Bear"), cast("Probe Ritual"), cast(koya)];
    let mut hook = |_: &Game, r: &Request| match &r.kind {
        RequestKind::UnlessPays { options, .. } => options.iter().find(|o| o.kind == ActionKind::DeclineCost).map(|o| Answer::Act(Action::of(o))),
        _ => None,
    };
    settle(&mut game, &steps, OVER.picking("Probe Mutant"), &mut hook);
    assert_eq!(names(&game, Zone::exile(YOU)), vec!["Probe Bear", "Probe Mutant"]);
    until_turn(&mut game, OVER.picking("Probe Mutant"), 2, &mut hook);
    assert_eq!(on_battlefield(&game, "Probe Bear").len(), 1);
    assert_eq!(on_battlefield(&game, "Probe Mutant").len(), 1);
}

/// Each card is returned at the next end step (CR 730.3c): Conciliator's Duelist's repartee.
#[test]
fn conciliators_duelist_returns_every_card_of_a_merged_creature() {
    let mut game = start(&["Probe Portal", "Conciliator's Duelist", "Probe Bear", "Probe Mutant", "Probe Nudge"], &[]);
    let steps = [cast("Probe Portal"), cast("Probe Mutant").mutating("Probe Bear"), cast("Probe Nudge").at("Conciliator's Duelist")];
    settle(&mut game, &steps, OVER.picking("Probe Mutant"), &mut no_hook());
    assert_eq!(names(&game, Zone::exile(YOU)), vec!["Probe Bear", "Probe Mutant"]);
    until_turn(&mut game, OVER, 2, &mut no_hook());
    assert_eq!(on_battlefield(&game, "Probe Bear").len(), 1);
    assert_eq!(on_battlefield(&game, "Probe Mutant").len(), 1);
}

/// A card of the merged permanent that is an Elf is not "another" Elf card in the graveyard (CR 730.3c): Morcant's Loyalist
/// has no other card to return.
#[test]
fn morcants_loyalist_returns_no_card_of_its_own_merged_permanent() {
    let mut game = start(&["Probe Portal", "Morcant's Loyalist", "Probe Mutant", "Probe Smite"], &[]);
    let steps = [cast("Probe Portal"), cast("Probe Mutant").mutating("Morcant's Loyalist"), cast("Probe Smite").at("Probe Mutant")];
    settle(&mut game, &steps, OVER, &mut no_hook());
    assert_eq!(in_graveyard(&game, &["Morcant's Loyalist", "Probe Mutant"]), 2, "neither Elf card went to hand");
}

/// Each card of the permanent may be cast from the graveyard as an Adventure (CR 730.3c): the Mosswood Dreadknight under a
/// Mutant is offered, though the Mutant's card is the top one.
#[test]
fn mosswood_dreadknight_may_be_cast_from_the_graveyard_under_a_mutant() {
    let mut game = start(&["Probe Portal", "Mosswood Dreadknight", "Probe Mold", "Probe Mutant", "Probe Ritual", "Probe Smite"], &[]);
    let steps = [
        cast("Probe Portal"),
        cast("Probe Mold"),
        cast("Probe Mutant").mutating("Mosswood Dreadknight"),
        cast("Probe Ritual"),
        cast("Probe Smite").at("Probe Mutant"),
    ];
    settle(&mut game, &steps, OVER, &mut no_hook());
    game.run();
    let Some(RequestKind::Priority { options }) = game.pending().map(|r| r.kind.clone()) else { panic!("priority") };
    assert!(
        options.iter().any(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == "Mosswood Dreadknight" && game.state.obj(s).zone == Zone::graveyard(YOU))),
        "the Dreadknight card may be cast from the graveyard: {options:?}"
    );
}

/// Each card of the permanent exiled returns when the attacking Phelia's trigger returns it, Phelia growing once as they
/// came back under your control (CR 730.3c).
#[test]
fn phelia_returns_every_card_of_a_merged_permanent() {
    let mut game = start(&["Probe Portal", "Phelia, Exuberant Shepherd", "Probe Bear", "Probe Mutant"], &[]);
    let steps = [cast("Probe Portal"), cast("Probe Mutant").mutating("Probe Bear")];
    settle(&mut game, &steps, OVER.picking("Probe Mutant"), &mut no_hook());
    let mut attack = |g: &Game, r: &Request| match &r.kind {
        RequestKind::DeclareAttackers { .. } if r.player == YOU => {
            let phelia = on_battlefield(g, "Phelia, Exuberant Shepherd")[0];
            Some(Answer::Attack(vec![(phelia, Target::Player(THEM))]))
        }
        _ => None,
    };
    run(&mut game, &[], OVER.picking("Probe Mutant"), &mut attack, &|g| g.state.turn.number >= 4);
    assert_eq!(on_battlefield(&game, "Probe Bear").len(), 1, "the Bear came back");
    assert_eq!(on_battlefield(&game, "Probe Mutant").len(), 1, "the Mutant came back");
    let phelia = on_battlefield(&game, "Phelia, Exuberant Shepherd")[0];
    assert_eq!(counters(&game, phelia, CounterKind::PlusOne), 1, "one counter: they entered under your control");
}

/// Each card is returned tapped and attacking (CR 730.3c, 508.4): The Neutrinos.
#[test]
fn the_neutrinos_return_every_card_of_a_merged_creature_attacking() {
    let mut game = start(&["Probe Portal", "The Neutrinos", "Probe Bear", "Probe Mutant"], &[]);
    let steps = [cast("Probe Portal"), cast("Probe Mutant").mutating("Probe Bear")];
    settle(&mut game, &steps, OVER.picking("Probe Mutant"), &mut no_hook());
    let mut attack = |g: &Game, r: &Request| match &r.kind {
        RequestKind::DeclareAttackers { .. } if r.player == YOU => {
            let neutrinos = on_battlefield(g, "The Neutrinos")[0];
            Some(Answer::Attack(vec![(neutrinos, Target::Player(THEM))]))
        }
        _ => None,
    };
    run(&mut game, &[], OVER.picking("Probe Mutant"), &mut attack, &|g| {
        g.state.turn.number == 3 && g.state.stack.is_empty() && g.state.triggers.is_empty() && g.state.attackers.len() >= 3
    });
    for name in ["Probe Bear", "Probe Mutant"] {
        let back = on_battlefield(&game, name);
        assert_eq!(back.len(), 1, "{name} came back");
        assert!(game.state.obj(back[0]).tapped && game.is_attacking(back[0]), "{name} is tapped and attacking");
    }
}

/// What each returned card attacks is chosen for that card, asked with it as the source, and it attacks what was chosen for it
/// (CR 508.4, 730.3c): The Neutrinos, the Bear sent at the player and the Mutant at the player's planeswalker.
#[test]
fn the_neutrinos_return_each_card_of_a_merged_creature_attacking_what_was_chosen_for_it() {
    let mut game = start(&["Probe Portal", "The Neutrinos", "Probe Bear", "Probe Mutant", "Probe Walker", "Probe Gift"], &["Probe Walker"]);
    let steps = [cast("Probe Portal"), cast("Probe Gift"), cast("Probe Mutant").mutating("Probe Bear")];
    settle(&mut game, &steps, OVER.picking("Probe Mutant"), &mut no_hook());
    let walker = on_battlefield(&game, "Probe Walker")[0];
    assert_eq!(game.state.obj(walker).controller, THEM);
    let mut asked_for = Vec::new();
    let mut hook = |g: &Game, r: &Request| match &r.kind {
        RequestKind::DeclareAttackers { .. } if r.player == YOU => {
            let neutrinos = on_battlefield(g, "The Neutrinos")[0];
            Some(Answer::Attack(vec![(neutrinos, Target::Player(THEM))]))
        }
        RequestKind::ChooseTarget { purpose: Purpose::EnterAttacking, source, .. } => {
            asked_for.push(g.name(*source));
            Some(Answer::Targets(vec![if g.name(*source) == "Probe Bear" { Target::Player(THEM) } else { Target::Object(walker) }]))
        }
        _ => None,
    };
    run(&mut game, &[], OVER.picking("Probe Mutant"), &mut hook, &|g| {
        g.state.turn.number == 3 && g.state.stack.is_empty() && g.state.triggers.is_empty() && g.state.attackers.len() >= 3
    });
    asked_for.sort_unstable();
    assert_eq!(asked_for, vec!["Probe Bear", "Probe Mutant"], "one choice for each card, naming it");
    for (name, attacked) in [("Probe Bear", Target::Player(THEM)), ("Probe Mutant", Target::Object(walker))] {
        let back = on_battlefield(&game, name);
        assert_eq!(back.len(), 1, "{name} came back");
        let attacker = game.state.attackers.iter().find(|a| a.id == back[0]).unwrap_or_else(|| panic!("{name} is attacking"));
        assert_eq!(attacker.target, attacked, "{name} attacks what was chosen for it");
    }
}

/// Every card of the permanent that was cast from the graveyard and gained "exile it and you gain 2 life" is exiled, and the life
/// is gained once (CR 730.3c): Serra Paragon's permission, the Mutant merged onto the Bear it let be cast.
#[test]
fn serra_paragon_exiles_every_card_of_a_merged_permanent_it_let_be_cast() {
    let hand = ["Probe Portal", "Serra Paragon", "Probe Mutant", "Probe Mill", "Probe Smite", "Island", "Island", "Probe Bear"];
    let mut game = start(&hand, &[]);
    let steps = [cast("Probe Portal"), cast("Probe Mill"), cast("Probe Mutant").mutating("Probe Bear"), cast("Probe Smite").at("Probe Mutant")];
    // The Bear, milled, is cast from the graveyard by Serra Paragon's permission as soon as it is offered.
    let mut hook = |g: &Game, r: &Request| match &r.kind {
        RequestKind::Priority { options } if r.player == YOU && g.state.stack.is_empty() => options
            .iter()
            .find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| g.name(s) == "Probe Bear" && g.state.obj(s).zone == Zone::graveyard(YOU)))
            .map(|o| Answer::Act(Action::of(o))),
        _ => None,
    };
    settle(&mut game, &steps, OVER.picking("Probe Bear"), &mut hook);
    assert_eq!(names(&game, Zone::exile(YOU)), vec!["Probe Bear", "Probe Mutant"], "both cards exiled");
    assert_eq!(game.state.players[YOU.0 as usize].life, 22, "2 life, once");
}

/// Craft returns the craft card transformed though another card is on top of the permanent it exiled, the materials linked to it
/// (CR 730.3c, 702.167a, 702.167c): Tithing Blade, made a creature by March of the Machines and mutated under a Mutant, which
/// stays in exile.
#[test]
fn craft_returns_the_craft_card_of_a_merged_permanent_transformed() {
    let mut game = start(&["Probe Portal", "March of the Machines", "Tithing Blade", "Grizzly Bears", "Probe Mutant", "Probe Ritual"], &[]);
    let steps = [cast("Probe Portal"), cast("Probe Mutant").mutating("Tithing Blade"), cast("Probe Ritual")];
    let mut crafted = false;
    let mut hook = |g: &Game, r: &Request| match &r.kind {
        RequestKind::ChooseCostPayment { choices, .. } => Some(Answer::CostPayment(choices.iter().position(|c| c.label.starts_with("Pay mana")).expect("mana can be paid before craft"))),
        RequestKind::Craft { cards, .. } => Some(Answer::Cards(cards.clone())),
        RequestKind::Priority { options } if r.player == YOU && g.state.stack.is_empty() && !crafted && count(g, Zone::graveyard(YOU), "Probe Ritual") == 1 => {
            let craft = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| g.name(s) == "Probe Mutant")).expect("the merged permanent can craft");
            crafted = true;
            Some(Answer::Act(Action::of(craft)))
        }
        _ => None,
    };
    run(&mut game, &steps, OVER.picking("Tithing Blade"), &mut hook, &|g| {
        !on_battlefield(g, "Consuming Sepulcher").is_empty() && g.state.stack.is_empty() && g.state.triggers.is_empty()
    });
    let sepulcher = on_battlefield(&game, "Consuming Sepulcher");
    assert_eq!(sepulcher.len(), 1, "the craft card returned transformed");
    assert_eq!(names(&game, Zone::exile(YOU)), vec!["Grizzly Bears", "Probe Mutant"], "the Mutant, no double-faced card, stays in exile");
    let materials: Vec<&str> = cantrip_mechanics::craft::exiled_cards_used_to_craft(&game, sepulcher[0]).into_iter().map(|c| game.name(c)).collect();
    assert_eq!(materials, vec!["Grizzly Bears"], "the material is linked to the permanent the craft card became");
}
