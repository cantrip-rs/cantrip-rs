//! "For as long as it has a [kind] counter on it" (CR 611.2b, `Duration::WhileCounter`) ended by the last such counter coming off
//! in the ways no printed card in the engine pairs with the duration: a stun counter removed in place of an untap (CR 122.1d),
//! ending a control-changing effect (`ops::end_counter_durations`, `settle_control`), counters removed as a cost (CR 118.3:
//! Loch Mare's), and damage removing a planeswalker's last loyalty counters (CR 120.3c). Test cards make the effects, beside the
//! printed cards and every other card the engine has.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

fn creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

fn the_creature(item: &StackItem) -> ObjectId {
    match item.target(0) {
        Target::Object(id) => id,
        other => panic!("a creature, not {other:?}"),
    }
}

/// "Tap target creature, put a stun counter on it, and gain control of it for as long as it has a stun counter on it."
fn seize(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let creature = the_creature(item);
    g.tap(creature);
    g.add_counters(creature, CounterKind::Stun, 1);
    g.gain_control(item.object, creature, item.controller, Duration::WhileCounter(CounterKind::Stun));
    Ok(())
}

/// "Target creature gets +3/+0 for as long as it has a -1/-1 counter on it."
fn mark(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.add_effect(item.object, the_creature(item), EffectKind::Pump(3, 0), Duration::WhileCounter(CounterKind::MinusOne));
    Ok(())
}

static SEIZE: CardDef = CardDef { name: "Probe Seize", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[target(creature)], seize), ..CardDef::BLANK };
static MARK: CardDef = CardDef { name: "Probe Mark", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[target(creature)], mark), ..CardDef::BLANK };

fn planeswalker(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Planeswalker))
}

/// "Gain control of target planeswalker for as long as it has a loyalty counter on it."
fn borrow(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(walker) = item.target(0) {
        g.gain_control(item.object, walker, item.controller, Duration::WhileCounter(CounterKind::Loyalty));
    }
    Ok(())
}

/// "Probe Bolt deals 3 damage to target planeswalker."
fn bolt_walker(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.damage(item.object, item.target(0), 3);
    Ok(())
}

/// "Whenever a planeswalker you control is put into a graveyard from the battlefield": judged as it last was (CR 603.10a),
/// "you" its controller then.
fn your_planeswalker_dies(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if from.is(ZoneKind::Battlefield) && to.is(ZoneKind::Graveyard) && q.state().obj(*old).controller == q.controller(source) =>
        {
            Some(Fired { subject: Target::None, data: [0, 0] })
        }
        _ => None,
    }
}

fn gain_one(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

/// A zero-cost planeswalker of loyalty 3, with no abilities.
static WALKER: CardDef = CardDef { name: "Probe Walker", types: TypeSet::of(&[CardType::Planeswalker]), loyalty: Some(3), ..CardDef::BLANK };
static BORROW: CardDef =
    CardDef { name: "Probe Borrow", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[target(planeswalker)], borrow), ..CardDef::BLANK };
static BOLT_WALKER: CardDef =
    CardDef { name: "Probe Bolt", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[target(planeswalker)], bolt_walker), ..CardDef::BLANK };
/// A zero-cost enchantment: "Whenever a planeswalker you control is put into a graveyard from the battlefield, you gain 1 life."
static MOURNER: CardDef = CardDef {
    name: "Probe Mourner",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, your_planeswalker_dies), gain_one, "Whenever a planeswalker you control is put into a graveyard from the battlefield, you gain 1 life.")],
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&SEIZE, &MARK, &WALKER, &BORROW, &BOLT_WALKER, &MOURNER])))
}

/// What a seat does, in order, each as soon as it is offered and its turn has come: cast a card or activate an ability
/// (its `index`) of a permanent, at a creature named.
enum Step {
    Cast(usize, u32, &'static str, Option<&'static str>),
    Activate(usize, u32, &'static str, u8, Option<&'static str>),
}

/// A game of the two hands (then Islands), played until every step is done and turn `until` has come. Every question is shown
/// to `watch` before it is answered.
fn play(hands: [&[&str]; 2], steps: Vec<Step>, until: u32, mut watch: impl FnMut(&Game)) -> Game {
    let names = |list: &[&str]| -> Vec<CardId> { list.iter().map(|n| table().find(n).unwrap_or_else(|| panic!("no card {n}"))).collect() };
    let deck = |hand: &[&str]| {
        let mut deck = names(hand);
        deck.extend(names(&vec!["Island"; 30 - hand.len()]));
        deck
    };
    let mut game = Game::new(table(), &Setup { decks: vec![deck(hands[0]), deck(hands[1])], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut steps = steps.into_iter().peekable();
    let named = |game: &Game, name: &str| game.state.battlefield().iter().copied().find(|&id| game.name(id) == name);
    for _ in 0..2000 {
        game.run();
        if steps.peek().is_none() && game.state.turn.number >= until && game.state.stack.is_empty() {
            return game;
        }
        let request = game.pending().cloned().expect("the game goes on");
        let turn = game.state.turn.number;
        watch(&game);
        let seat = request.player.0 as usize;
        let answer = match &request.kind {
            RequestKind::Priority { options } => {
                let wanted = match steps.peek() {
                    Some(&Step::Cast(s, at, name, aimed)) if s == seat && turn >= at && game.state.stack.is_empty() => {
                        Some((ActionKind::CastSpell, name, 0, aimed))
                    }
                    Some(&Step::Activate(s, at, name, index, aimed)) if s == seat && turn >= at && game.state.stack.is_empty() => {
                        Some((ActionKind::ActivateAbility, name, index, aimed))
                    }
                    _ => None,
                };
                let option = wanted.and_then(|(kind, name, index, _)| {
                    options.iter().find(|o| o.kind == kind && (kind == ActionKind::CastSpell || o.index == index) && o.source.is_some_and(|s| game.name(s) == name))
                });
                match (option, wanted) {
                    (Some(option), Some((_, _, _, aimed))) => {
                        steps.next();
                        let mut action = Action::of(option);
                        if let Some(aimed) = aimed
                            && option.targets_required > 0
                        {
                            action.targets = vec![Target::Object(named(&game, aimed).expect("the target"))];
                        }
                        Answer::Act(action)
                    }
                    _ => Answer::Pass,
                }
            }
            RequestKind::PayCosts { options, .. } => {
                let mox = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s).text().starts_with("Mox")));
                let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting);
                Answer::Act(Action::of(confirm.or(mox).expect("a Mox to pay with, or the payment to confirm")))
            }
            RequestKind::ChooseTarget { candidates, .. } => Answer::Targets(vec![candidates[0]]),
            RequestKind::RemoveCounters { pools, total, .. } => Answer::Counters(vec![(pools[0].object, pools[0].kind, u16::from(*total))]),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the game went on too long");
}

/// Bot1 takes Bot2's Ornithopter with Probe Seize on turn 3: tapped, stunned, Bot1's for as long as it has the stun counter. At
/// Bot1's untap step on turn 5 the counter comes off in place of the untap (CR 122.1d), so the control effect ends and the
/// Ornithopter, still tapped, is Bot2's again.
#[test]
fn a_stun_counter_removed_in_place_of_an_untap_ends_control_for_as_long_as_it_has_it() {
    let mut seized = false;
    let game = play(
        [&["Probe Seize"], &["Ornithopter"]],
        vec![Step::Cast(1, 2, "Ornithopter", None), Step::Cast(0, 3, "Probe Seize", Some("Ornithopter"))],
        5,
        |game| {
            if let Some(&thopter) = game.state.battlefield().iter().find(|&&id| game.name(id) == "Ornithopter")
                && game.state.turn.number == 4
            {
                seized |= game.state.obj(thopter).controller == PlayerId(0) && game.state.obj(thopter).counters(CounterKind::Stun) == 1;
            }
        },
    );
    assert!(seized, "Bot1 controlled the stunned Ornithopter through Bot2's turn");
    let thopter = *game.state.battlefield().iter().find(|&&id| game.name(id) == "Ornithopter").expect("the Ornithopter");
    let obj = game.state.obj(thopter);
    assert_eq!(obj.controller, PlayerId(1), "the control effect outlived the stun counter");
    assert_eq!(obj.counters(CounterKind::Stun), 0);
    assert!(obj.tapped, "the untap the stun counter replaced happened");
}

/// Loch Mare enters with three -1/-1 counters (a 1/2) and Probe Mark gives it +3/+0 while it has one. Its abilities remove two
/// and then the last as costs (CR 118.3): with one left it is a 6/4, and with none the +3/+0 is gone - a 4/5.
#[test]
fn counters_removed_as_a_cost_end_an_effect_for_as_long_as_it_has_one() {
    let power = |game: &Game| {
        let mare = *game.state.battlefield().iter().find(|&&id| game.name(id) == "Loch Mare").expect("Loch Mare");
        let c = game.chars(mare);
        (c.power.unwrap(), c.toughness.unwrap(), game.state.obj(mare).counters(CounterKind::MinusOne))
    };
    let mut seen = Vec::new();
    let game = play(
        [&["Mox Sapphire", "Mox Sapphire", "Mox Jet", "Mox Ruby", "Mox Pearl", "Loch Mare", "Probe Mark"], &[]],
        vec![
            Step::Cast(0, 1, "Mox Sapphire", None),
            Step::Cast(0, 1, "Mox Sapphire", None),
            Step::Cast(0, 1, "Mox Jet", None),
            Step::Cast(0, 1, "Mox Ruby", None),
            Step::Cast(0, 1, "Mox Pearl", None),
            Step::Cast(0, 1, "Loch Mare", None),
            Step::Cast(0, 1, "Probe Mark", Some("Loch Mare")),
            Step::Activate(0, 3, "Loch Mare", 1, Some("Loch Mare")),
            Step::Activate(0, 3, "Loch Mare", 0, None),
        ],
        3,
        |game| {
            if game.state.battlefield().iter().any(|&id| game.name(id) == "Loch Mare") {
                let now = power(game);
                if seen.last() != Some(&now) {
                    seen.push(now);
                }
            }
        },
    );
    assert_eq!(power(&game), (4, 5, 0));
    assert!(seen.contains(&(4, 2, 3)) && seen.contains(&(6, 4, 1)), "Loch Mare was not 4/2 and then 6/4 on the way: {seen:?}");
}

/// Bot2 gains control of Bot1's Probe Walker for as long as it has a loyalty counter on it; Bot1's Probe Bolt then deals it 3,
/// removing its last 3 loyalty counters (CR 120.3c). The control effect ends with them, before the state-based actions, so the
/// Walker is Bot1's again as it is put into its owner's graveyard (CR 704.5i): Bot1's Mourner triggers, not Bot2's.
#[test]
fn damage_removing_the_last_loyalty_counter_ends_control_for_as_long_as_it_has_one() {
    let game = play(
        [&["Probe Walker", "Probe Mourner", "Probe Bolt"], &["Probe Mourner", "Probe Borrow"]],
        vec![
            Step::Cast(0, 1, "Probe Walker", None),
            Step::Cast(0, 1, "Probe Mourner", None),
            Step::Cast(1, 2, "Probe Mourner", None),
            Step::Cast(1, 2, "Probe Borrow", Some("Probe Walker")),
            Step::Cast(0, 3, "Probe Bolt", Some("Probe Walker")),
        ],
        3,
        |_| {},
    );
    assert!(game.state.zone(Zone::graveyard(PlayerId(0))).iter().any(|&id| game.name(id) == "Probe Walker"), "the Walker did not die");
    assert_eq!((game.state.player(PlayerId(0)).life, game.state.player(PlayerId(1)).life), (21, 20), "the Walker died under the wrong control");
}
