//! A damage event's life results replaced as one event's (CR 120.3a, 120.4c, 616.1), over test cards printing the shared
//! "damage that would reduce your life total to less than 1 reduces it to 1 instead" (Ali from Cairo, Worship), "if you
//! would gain life, you gain twice that much life instead" (Boon Reflection) and a prevention effect that gains the
//! damage it prevented (Awe Strike, Reverse Damage): the loss knows it is damage's, each result sees the others of the
//! event, the affected player chooses which applies first, and the prevention's gain is one of the event's results (CR
//! 120.4's Worship and Awe Strike example, read with CR 615.5).

use crate::ops::{change_life, damage, damage_together};
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_mechanics::{prevention, replacements};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static ALI: CardDef = CardDef {
    name: "Damage Life Results Ali",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacements::DAMAGE_CANT_REDUCE_YOUR_LIFE_BELOW_1],
    ..CardDef::BLANK
};

static BOON: CardDef = CardDef {
    name: "Damage Life Results Boon Reflection",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacements::GAIN_TWICE_THAT_MUCH_LIFE],
    ..CardDef::BLANK
};

static GIANT: CardDef = CardDef {
    name: "Damage Life Results Giant",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(5),
    toughness: Some(5),
    ..CardDef::BLANK
};

static LIFELINKER: CardDef = CardDef {
    name: "Damage Life Results Lifelinker",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    keywords: Keywords::of(&[Keyword::Lifelink]),
    ..CardDef::BLANK
};

/// Awe Strike's effect: "The next time target creature would deal damage this turn, prevent that damage. You gain life
/// equal to the damage prevented this way."
static AWE_STRIKE: CardDef = CardDef {
    name: "Damage Life Results Awe Strike",
    types: TypeSet::of(&[CardType::Instant]),
    granted_preventions: &[GrantedPreventionDef {
        gain_prevented: true,
        text: "The next time target creature would deal damage this turn, prevent that damage. You gain life equal to the damage prevented this way.",
        ..prevention::DAMAGE_DEALT_BY
    }],
    ..CardDef::BLANK
};

struct Table {
    game: Game,
    /// The resolved Awe Strike, in YOU's graveyard.
    strike: ObjectId,
    /// Two 5/5 creatures THEM controls.
    giant: ObjectId,
    other: ObjectId,
    /// A 3/3 lifelink creature YOU control.
    lifelinker: ObjectId,
}

fn table(you: &[&'static CardDef]) -> Table {
    let cards: &'static CardTable = Box::leak(Box::new(CardTable::new([&ALI, &BOON, &GIANT, &LIFELINKER, &AWE_STRIKE])));
    let mut game = Game::new(cards, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    // Turn 1: no turn of the game is one in which damage can't be prevented (`no_prevention_turn` starts at 0).
    game.state.update_turn(|t| t.number = 1);
    let put = |game: &mut Game, card: &'static CardDef, owner: PlayerId, zone: Zone| {
        let id = game.state.push_obj(Obj::new(cards.id_of(card), owner, zone));
        game.state.zone_push(zone, id);
        id
    };
    for &card in you {
        put(&mut game, card, YOU, Zone::BATTLEFIELD);
    }
    let strike = put(&mut game, &AWE_STRIKE, YOU, Zone::graveyard(YOU));
    let giant = put(&mut game, &GIANT, THEM, Zone::BATTLEFIELD);
    let other = put(&mut game, &GIANT, THEM, Zone::BATTLEFIELD);
    let lifelinker = put(&mut game, &LIFELINKER, YOU, Zone::BATTLEFIELD);
    game.state.see_statics();
    game.state.update_player(YOU, |p| p.life = 2);
    Table { game, strike, giant, other, lifelinker }
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// A frame to ask questions in, its answers given in advance: what the run reads back as it asks (`Cx::ask_core`).
fn answered(cx: &mut Cx, answers: &[Answer]) {
    let frame = cx.s().agenda.len();
    cx.st().frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::BeginStep));
    for answer in answers {
        cx.st().update_frame(frame, |f| f.answers.push((None, answer.clone())));
    }
    cx.frame = frame;
}

/// Awe Strike's effect about `creature`, made by YOU.
fn awe_strike(cx: &mut Cx, strike: ObjectId, creature: ObjectId) {
    let card = cx.s().obj(strike).card;
    cantrip_api::Game::add_effect(cx, strike, creature, EffectKind::Prevention { card, index: 0, to: Target::None, shield: Shield::NextTime }, Duration::EndOfTurn);
}

fn changes(cx: &Cx, player: PlayerId) -> Vec<i32> {
    cx.g.log.iter().filter_map(|r| match r.event {
        Event::LifeChange { player: p, delta } if p == player => Some(delta),
        _ => None,
    }).collect()
}

fn dealt_to(cx: &Cx, player: PlayerId) -> u32 {
    cx.g.log.iter().filter_map(|r| match r.event {
        Event::Damage { target: Target::Player(p), amount, .. } if p == player => Some(amount),
        _ => None,
    }).sum()
}

/// CR 120.3a, 120.4c, 616.1: at 2 life, dealt 5 damage while their lifelink creature deals 3, the player has two results
/// of one event, [lose 5, gain 3], and two effects on them, and chooses which applies first. The floor first sees the
/// event take them to 0 and leaves them at 1 with the gain as it then is (a loss of 4); Boon Reflection then doubles the
/// gain: 4 life. Boon Reflection first makes the event [lose 5, gain 6], which no longer reduces them below 1: 3 life.
#[test]
fn the_affected_player_orders_a_life_floor_on_the_loss_with_a_gain_replacement_on_lifelink() {
    for (first, loss, gain, life) in [(0, -4, 6, 4), (1, -5, 6, 3)] {
        let Table { mut game, giant, lifelinker, .. } = table(&[&ALI, &BOON]);
        let mut cx = cx(&mut game);
        answered(&mut cx, &[Answer::CostPayment(first)]);
        damage_together(&mut cx, &[(giant, Target::Player(YOU), 5), (lifelinker, Target::Player(THEM), 3)]);
        assert_eq!(cx.cursor, 1, "the order of the two results' effects was asked");
        assert_eq!(dealt_to(&cx, YOU), 5, "all the damage is dealt: only the life it costs changes");
        assert_eq!(changes(&cx, YOU), [loss, gain]);
        assert_eq!(cx.s().player(YOU).life, life);
    }
}

/// The question names each of the player's results of the event, with its amount (`ReplacementOccurrence::Life`).
#[test]
fn the_order_question_names_the_events_life_results() {
    let Table { mut game, giant, lifelinker, .. } = table(&[&ALI, &BOON]);
    let mut cx = cx(&mut game);
    answered(&mut cx, &[]);
    damage_together(&mut cx, &[(giant, Target::Player(YOU), 5), (lifelinker, Target::Player(THEM), 3)]);
    let asked = cx.s().agenda[cx.frame].pending.clone().expect("a question was asked");
    assert_eq!(asked.player, YOU, "the affected player chooses (CR 616.1)");
    let RequestKind::OrderReplacementOccurrences { occurrences } = &asked.kind else { panic!("{:?}", asked.kind) };
    assert!(matches!(occurrences[..], [
        ReplacementOccurrence::Life { player: YOU, amount: -5, .. },
        ReplacementOccurrence::Life { player: YOU, amount: 3, .. },
    ]), "{occurrences:?}");
}

/// CR 120.4's example: at 2 life, two unblocked 5/5s and Awe Strike on one of them make the event [5 damage is dealt to
/// the player, the player gains 5 life], processed into [lose 5, gain 5]. The floor sees the event would not reduce the
/// life total to less than 1 and does not apply: 2 life. (Were the gain after the event, CR 615.5, the floor would have
/// left 1 and the gain made 6.)
#[test]
fn a_prevention_effects_gain_is_a_result_of_the_damage_event_it_prevented() {
    let Table { mut game, strike, giant, other, .. } = table(&[&ALI]);
    let mut cx = cx(&mut game);
    awe_strike(&mut cx, strike, giant);
    assert_eq!(damage_together(&mut cx, &[(giant, Target::Player(YOU), 5), (other, Target::Player(YOU), 5)]), [0, 5]);
    assert_eq!(changes(&cx, YOU), [-5, 5], "one loss and one gain, announced together, not netted");
    assert_eq!(cx.s().player(YOU).life, 2);
    assert!(!cx.s().effects.iter().any(|e| matches!(e.kind, EffectKind::Prevention { .. })), "the next time is used up (CR 615.8)");
}

/// The gain is the event's even where the effect prevents all of its damage: the event is [the player gains 5 life].
/// Boon Reflection doubles it as a result of the event (CR 120.4c).
#[test]
fn a_prevention_gain_happens_where_all_the_damage_is_prevented() {
    let Table { mut game, strike, giant, .. } = table(&[&BOON]);
    let mut cx = cx(&mut game);
    awe_strike(&mut cx, strike, giant);
    assert_eq!(damage(&mut cx, giant, Target::Player(YOU), 5, false), 0);
    assert_eq!(changes(&cx, YOU), [10]);
    assert_eq!(cx.s().player(YOU).life, 12);
}

/// The rulings: the floor is about damage. An effect's loss is not (CR 120.3a): it takes the player below 1. Damage is:
/// it leaves them at 1, with all of it dealt. A player at 1 is dealt damage that costs no life - no life change at all
/// (CR 119.10) - and one with less than 1 is dealt damage as normal.
#[test]
fn the_floor_knows_a_loss_is_damages_result() {
    let Table { mut game, giant, .. } = table(&[&ALI]);
    let mut cx = cx(&mut game);
    change_life(&mut cx, YOU, -5);
    assert_eq!(cx.s().player(YOU).life, -3, "an effect's loss is no damage");
    cx.st().update_player(YOU, |p| p.life = 4);
    damage(&mut cx, giant, Target::Player(YOU), 5, false);
    assert_eq!(cx.s().player(YOU).life, 1);
    assert_eq!(changes(&cx, YOU), [-5, -3]);
    damage(&mut cx, giant, Target::Player(YOU), 5, false);
    assert_eq!(cx.s().player(YOU).life, 1);
    assert_eq!(changes(&cx, YOU), [-5, -3], "a loss of 0 is no event");
    assert_eq!(dealt_to(&cx, YOU), 10);
    cx.st().update_player(YOU, |p| p.life = 0);
    damage(&mut cx, giant, Target::Player(YOU), 5, false);
    assert_eq!(cx.s().player(YOU).life, -5);
}
