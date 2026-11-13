//! The cards whose "each player / each opponent loses N life" (and Memory Jar's "each player discards their hand") is one
//! instruction for several players: their results are one simultaneous event (CR 101.4, 603.2c), not one event per player.
//! A "whenever one or more players lose life" or "whenever one or more cards are discarded" ability (Emet-Selch of the
//! Third Seat's kind) triggers once in a three-player game. No implemented card has such a trigger yet, so a test card
//! watches here; each card's own effect runs - the card itself at no cost where it can be cast or activated at once, and
//! otherwise its triggered ability's effect on a zero-cost test sorcery with the ability's targets.
//! `each_opponent_loses_life_unless_together` is Ob Nixilis, the Adversary's. Rottenmouth Viper's losses and an exchange of
//! life totals (`Game::together`) are one event in the same way.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

/// Any life loss, whoever suffers it.
fn a_player_loses_life(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::LifeChange { delta, .. } if *delta < 0 => Fired::PLAIN,
        _ => None,
    }
}

/// Any card discarded, whoever discards it.
fn a_card_is_discarded(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Discarded { .. } => Fired::PLAIN,
        _ => None,
    }
}

fn gain_one(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

/// A zero-cost enchantment: "Whenever one or more players lose life, you gain 1 life."
static LIFE_WITNESS: CardDef = CardDef {
    name: "Probe Life Witness",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when(EventMask::LIFE_CHANGE, a_player_loses_life), gain_one, "Whenever one or more players lose life, you gain 1 life.")
        .once_per_batch()],
    ..CardDef::BLANK
};

/// A zero-cost enchantment: "Whenever one or more cards are discarded, you gain 1 life."
static DISCARD_WITNESS: CardDef = CardDef {
    name: "Probe Discard Witness",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when(EventMask::DISCARDED, a_card_is_discarded), gain_one, "Whenever one or more cards are discarded, you gain 1 life.")
        .once_per_batch()],
    ..CardDef::BLANK
};

static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// A colorless artifact of mana value 2: what Parker Luck's players reveal.
static ROCK: CardDef =
    CardDef { name: "Probe Rock", cost: ManaCost { generic: 2, ..ManaCost::ZERO }, types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };

fn real(name: &str) -> &'static CardDef {
    cantrip_cards::all().into_iter().find(|c| c.name == name).unwrap_or_else(|| panic!("{name} is implemented"))
}

/// The card itself, castable for nothing.
fn free(name: &str) -> &'static CardDef {
    Box::leak(Box::new(CardDef { cost: ManaCost::ZERO, ..*real(name) }))
}

/// A zero-cost sorcery that does what `card`'s triggered ability starting `text` does, with its targets.
fn probe(card: &str, text: &str) -> &'static CardDef {
    let def = real(card);
    let (effect, targets) = def
        .abilities
        .iter()
        .chain(def.delayed)
        .find_map(|a| match a {
            AbilityDef::Triggered(t) if t.text.starts_with(text) => Some((t.effect, t.targets)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{card} has \"{text}\""));
    Box::leak(Box::new(CardDef { name: "Probe", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(targets, effect), ..CardDef::BLANK }))
}

/// Bot1 (seat 0) puts `witness` onto the battlefield, then casts `card` (or activates it, once it is on the battlefield),
/// answering every question plainly: a "may" no, an optional discard none, a forced choice the first cards it may take,
/// targets the opponents. `others` is each opponent's library. Played until everything has resolved and `done` holds.
fn play(witness: &'static CardDef, card: &'static CardDef, activate: bool, others: Vec<&'static CardDef>, done: fn(&Game) -> bool) -> Game {
    // Rottenmouth Viper's free copy names its choices by the real card's labels, which the table must hold.
    let cards: Vec<&'static CardDef> = vec![witness, card, &FILLER, &ROCK, real("Rottenmouth Viper")];
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new(cards)));
    let find = |c: &CardDef| table.find(c.name).unwrap();
    let mut first = vec![find(witness), find(card)];
    first.extend(std::iter::repeat_n(find(&FILLER), 28));
    let other: Vec<_> = others.iter().map(|&c| find(c)).collect();
    let mut game = Game::new(table, &Setup { decks: vec![first, other.clone(), other], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut plan = vec![(witness.name, ActionKind::CastSpell), (card.name, ActionKind::CastSpell)];
    if activate {
        plan.push((card.name, ActionKind::ActivateAbility));
    }
    let mut plan = plan.into_iter().peekable();
    for _ in 0..200 {
        game.run();
        if plan.peek().is_none()
            && game.state.stack.is_empty()
            && game.state.triggers.is_empty()
            && !game.pending().is_some_and(|r| matches!(r.kind, RequestKind::OrderTriggers { .. }))
            && done(&game)
        {
            return game;
        }
        let request = game.pending().cloned().expect("the game goes on");
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) && game.state.stack.is_empty() => {
                match plan.peek().and_then(|&(n, kind)| options.iter().find(|o| o.kind == kind && o.source.is_some_and(|s| game.name(s) == n))) {
                    Some(option) => {
                        plan.next();
                        Answer::Act(Action::of(option))
                    }
                    None => Answer::Pass,
                }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::May { .. } => Answer::No,
            RequestKind::ChooseCards { cards, rule, .. } => match rule.count {
                Count::UpTo(_) | Count::AnyNumber => Answer::Cards(Vec::new()),
                count => {
                    let choosable = rule.among.clone().unwrap_or_else(|| cards.clone());
                    Answer::Cards(choosable.into_iter().take(count.most().unwrap_or(0) as usize).collect())
                }
            },
            RequestKind::ChooseTarget { candidates, .. } => {
                Answer::Targets(candidates.iter().copied().filter(|&t| matches!(t, Target::Player(p) if p != PlayerId(0))).collect())
            }
            RequestKind::OrderTriggers { sources, .. } => Answer::Order((0..count_u32(sources.len())).collect()),
            // Rottenmouth Viper's "sacrifice, discard or lose 4 life": the last, losing the life.
            RequestKind::ChooseModes { eligible, .. } => Answer::Modes(vec![eligible.len() as u8 - 1]),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the game did not finish");
}

fn all_resolved(_: &Game) -> bool {
    true
}

fn lives(game: &Game) -> [i32; 3] {
    [0, 1, 2].map(|p| game.state.player(PlayerId(p)).life)
}

/// "When this creature enters, draw a card. Each player loses 1 life."
#[test]
fn conciliators_duelist_each_player_loses_1_in_one_event() {
    let game = play(&LIFE_WITNESS, free("Conciliator's Duelist"), false, vec![&FILLER; 30], all_resolved);
    assert_eq!(lives(&game), [20 - 1 + 1, 19, 19]);
}

/// III - Hall of Sorrow: "Draw two cards. Each player loses 2 life."
#[test]
fn summon_primal_odin_hall_of_sorrow_each_player_loses_2_in_one_event() {
    let game = play(&LIFE_WITNESS, probe("Summon: Primal Odin", "III"), false, vec![&FILLER; 30], all_resolved);
    assert_eq!(lives(&game), [20 - 2 + 1, 18, 18]);
}

/// "Each player loses half their life, then discards half the cards in their hand, ..."
#[test]
fn pox_plague_each_player_loses_half_their_life_in_one_event() {
    let game = play(&LIFE_WITNESS, free("Pox Plague"), false, vec![&FILLER; 30], all_resolved);
    assert_eq!(lives(&game), [10 + 1, 10, 10]);
}

/// "Whenever Kroxa enters or attacks, each opponent discards a card, then each opponent who didn't discard a nonland card this
/// way loses 3 life." Both discard a land.
#[test]
fn kroxa_each_opponent_who_discarded_a_land_loses_3_in_one_event() {
    let game = play(&LIFE_WITNESS, free("Kroxa, Titan of Death's Hunger"), false, vec![&FILLER; 30], all_resolved);
    assert_eq!(lives(&game), [20 + 1, 17, 17]);
}

/// Every opponent discards their hand, then Entropic Battlecruiser's attack trigger's effect.
fn opponents_empty_their_hands_then_battlecruiser(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    static EFFECT: OnceLock<Effect> = OnceLock::new();
    let effect = EFFECT.get_or_init(|| probe("Entropic Battlecruiser", "Whenever this Spacecraft attacks").spell.unwrap().effect);
    let hands: Vec<ObjectId> = g.state().opponents(item.controller).flat_map(|p| g.state().zone(Zone::hand(p)).to_vec()).collect();
    g.discard_together(&hands);
    g.end_sentence();
    effect(g, item)
}

/// "Whenever this Spacecraft attacks, each opponent discards a card. Each opponent who can't loses 3 life." Both opponents
/// have empty hands.
#[test]
fn entropic_battlecruiser_each_opponent_who_cant_discard_loses_3_in_one_event() {
    let card = Box::leak(Box::new(CardDef { spell: spell(&[], opponents_empty_their_hands_then_battlecruiser), ..*probe("Entropic Battlecruiser", "Whenever this Spacecraft attacks") }));
    let game = play(&LIFE_WITNESS, card, false, vec![&FILLER; 30], all_resolved);
    assert!(game.state.zone(Zone::hand(PlayerId(1))).is_empty() && game.state.zone(Zone::hand(PlayerId(2))).is_empty());
    assert_eq!(lives(&game), [20 + 1, 17, 17]);
}

/// II: "Each player may discard a card. Each player who doesn't loses 3 life." Nobody discards.
#[test]
fn the_death_of_gwen_stacy_each_player_who_doesnt_discard_loses_3_in_one_event() {
    let game = play(&LIFE_WITNESS, probe("The Death of Gwen Stacy", "II"), false, vec![&FILLER; 30], all_resolved);
    assert_eq!(lives(&game), [20 - 3 + 1, 17, 17]);
}

/// "Whenever a legendary creature you control enters, any number of target players each mill a card and lose 1 life." Both
/// opponents are targeted.
#[test]
fn tinybones_joins_up_target_players_lose_1_in_one_event() {
    let game = play(&LIFE_WITNESS, probe("Tinybones Joins Up", "Whenever a legendary creature"), false, vec![&FILLER; 30], all_resolved);
    assert_eq!(lives(&game), [20 + 1, 19, 19]);
}

/// "Two target players each reveal the top card of their library. They each lose life equal to the mana value of the card
/// revealed by the other player." Both opponents reveal a card of mana value 2.
#[test]
fn parker_luck_both_players_lose_life_in_one_event() {
    let game = play(&LIFE_WITNESS, probe("Parker Luck", "At the beginning of your end step"), false, vec![&ROCK; 30], all_resolved);
    assert_eq!(lives(&game), [20 + 1, 18, 18]);
}

/// "Whenever this creature enters or attacks, put a blight counter on it. Then for each blight counter on it, each opponent
/// loses 4 life unless that player sacrifices a nonland permanent of their choice or discards a card." Both opponents choose
/// the life: every loss is one simultaneous action with the sacrifices and discards (the rulings, `Game::together`).
#[test]
fn rottenmouth_viper_each_opponents_4_life_in_one_event() {
    let viper = Box::leak(Box::new(CardDef { cost: ManaCost::ZERO, additional_cost: None, ..*real("Rottenmouth Viper") }));
    let game = play(&LIFE_WITNESS, viper, false, vec![&FILLER; 30], all_resolved);
    assert_eq!(lives(&game), [20 + 1, 16, 16]);
}

/// Bot2 loses 5 life, then Bot1 exchanges life totals with Bot2 (CR 701.12c): Bot1 loses 5 and Bot2 gains 5, one simultaneous
/// change of both totals announced together, not a loss and then a gain.
fn bot2_loses_5_then_exchange_with_bot2(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.lose_life(PlayerId(1), 5);
    g.end_sentence();
    cantrip_mechanics::exchange::life_totals(g, item.controller, PlayerId(1));
    Ok(())
}

/// Any loss of life, or a gain by anyone but Bot1 (whose own gains, the witness's, it does not watch).
fn a_loss_or_anothers_gain(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::LifeChange { player, delta } if *delta < 0 || *player != PlayerId(0) => Fired::PLAIN,
        _ => None,
    }
}

/// A zero-cost enchantment: "Whenever one or more players lose life or an opponent gains life, you gain 1 life."
static CHANGE_WITNESS: CardDef = CardDef {
    name: "Probe Change Witness",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when(EventMask::LIFE_CHANGE, a_loss_or_anothers_gain), gain_one, "Whenever one or more players lose life or an opponent gains life, you gain 1 life.")
        .once_per_batch()],
    ..CardDef::BLANK
};

/// An exchange of life totals (Mister Negative's) is one event: the witness triggers once for Bot2's 5 lost and once for the
/// exchange's loss and gain together (20 and 15 exchanged, then 2 gained), not once for each.
#[test]
fn an_exchange_of_life_totals_is_one_event() {
    let card = Box::leak(Box::new(CardDef { name: "Probe", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], bot2_loses_5_then_exchange_with_bot2), ..CardDef::BLANK }));
    let game = play(&CHANGE_WITNESS, card, false, vec![&FILLER; 30], all_resolved);
    assert_eq!(lives(&game), [15 + 2, 20, 20]);
}

/// Memory Jar's end step: "each player discards their hand" is one discard of every hand.
#[test]
fn memory_jar_every_hand_is_discarded_in_one_event() {
    let game = play(&DISCARD_WITNESS, free("Memory Jar"), true, vec![&FILLER; 30], |g| g.state.player(PlayerId(0)).life != 20);
    assert_eq!(lives(&game), [20 + 1, 20, 20]);
    for p in 0..3 {
        assert!(!game.state.zone(Zone::graveyard(PlayerId(p))).is_empty(), "Bot{} discarded a hand", p + 1);
    }
}
