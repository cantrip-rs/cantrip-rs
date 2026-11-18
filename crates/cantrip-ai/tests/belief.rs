//! Worlds dealt by the belief head (`cantrip_ai::belief`, `Prior::with_hand`): an opponent's hidden hand drawn by the
//! weights, their other unseen cards uniformly from what is left; every world still one the seat's knowledge allows,
//! and the same world for the same generator. The tiny model's belief head makes weights for the opponent's pool.

use std::collections::BTreeMap;
use std::path::Path;

use cantrip_ai::belief::{self, HandWeights, MAX_WEIGHT, MIN_WEIGHT, at_least_one};
use cantrip_ai::record::table;
use cantrip_ai::{Knowledge, Prior, TractEvaluator, determinize};
use cantrip_engine::*;
use cantrip_obs::InfoMode;
use cantrip_script::GameScript;

const BOT1: PlayerId = PlayerId(0);
const BOT2: PlayerId = PlayerId(1);

/// Bot2's opening hand is seven Lightning Bolts, hidden from Bot1; its unseen cards are ten Bolts and twenty Mountains.
fn game() -> Game {
    let text = "[Bot1 Deck]
30 Mountain

[Bot2 Deck]
10 Lightning Bolt
20 Mountain

[Actions]
Bot1.Hand.Mountain.Play
";
    let script = GameScript::parse(text, "belief").expect("parses");
    let game = cantrip_script::run(&script).map_err(|f| f.report).expect("plays").game;
    assert_eq!(game.pending().map(|r| r.player), Some(BOT1));
    game
}

fn bolt() -> CardId {
    card("Lightning Bolt").unwrap()
}

/// The Bolts a world dealt Bot2's hand, and Bot2's hand and library as one multiset of names.
fn dealt(game: &Game, prior: &Prior, seed: u64) -> (usize, BTreeMap<&'static str, usize>) {
    let mut world = game.fork_for_search();
    world.reseed(seed);
    determinize(&mut world, &Knowledge::of(game, BOT1), BOT1, InfoMode::Decklist, prior, &mut Rng::new(seed));
    let name = |id: &ObjectId| world.cards.get(world.state.obj(*id).card).name;
    let hand = world.state.zone(Zone::hand(BOT2));
    let bolts = hand.iter().filter(|&&id| name(&id) == "Lightning Bolt").count();
    let mut all = BTreeMap::new();
    for id in hand.iter().chain(world.state.zone(Zone::library(BOT2))) {
        *all.entry(name(id)).or_default() += 1;
    }
    (bolts, all)
}

fn weights(card: CardId, w: f64) -> HandWeights {
    let mut h = HandWeights::default();
    h.seats.entry(BOT2).or_default().insert(card, w);
    h
}

#[test]
fn hands_are_dealt_by_the_weights_and_worlds_stay_possible() {
    let game = game();
    let uniform = Prior::new();
    let bolts_in = Prior::new().with_hand(weights(bolt(), MAX_WEIGHT));
    let bolts_out = Prior::new().with_hand(weights(bolt(), MIN_WEIGHT));
    let mean = |prior: &Prior| {
        let mut sum = 0;
        for seed in 0..40 {
            let (b, all) = dealt(&game, prior, seed);
            assert_eq!(all.get("Lightning Bolt"), Some(&10), "every world holds Bot2's ten Bolts");
            assert_eq!(all.get("Mountain"), Some(&20));
            sum += b;
        }
        sum as f64 / 40.0
    };
    let (u, i, o) = (mean(&uniform), mean(&bolts_in), mean(&bolts_out));
    // Uniformly, 7 x 10 / 30 = 2.33 Bolts a hand.
    assert!((1.2..=3.5).contains(&u), "uniform {u}");
    assert!(i >= 6.5, "weighted towards Bolts {i}");
    assert!(o <= 0.3, "weighted away from Bolts {o}");
    // The same generator deals the same world.
    assert_eq!(dealt(&game, &bolts_in, 3), dealt(&game, &bolts_in, 3));
    // No weights at all is the deal as before.
    assert_eq!(dealt(&game, &Prior::new().with_hand(HandWeights::default()), 7), dealt(&game, &uniform, 7));
    assert!(Prior::new().with_hand(HandWeights::default()).hand().is_none());
}

#[test]
fn the_tiny_models_belief_weighs_the_opponents_pool() {
    let game = game();
    let tiny = TractEvaluator::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/tiny"), table()).unwrap();
    let request = game.pending().cloned().unwrap();
    let k = Knowledge::of(&game, BOT1);
    let w = belief::hand_weights(&game, &request, &k, InfoMode::Decklist, &tiny, table()).unwrap().expect("weights for Bot2");
    let seat = w.seats.get(&BOT2).expect("Bot2's pool");
    assert!(seat.contains_key(&bolt()));
    assert!(seat.values().all(|&x| (MIN_WEIGHT..=MAX_WEIGHT).contains(&x)), "{seat:?}");
    assert!(belief::hand_weights(&game, &request, &k, InfoMode::Full, &tiny, table()).unwrap().is_none(), "no pools in Full mode");
}

#[test]
fn at_least_one_is_the_hypergeometric_chance() {
    assert_eq!(at_least_one(30, 10, 0), 0.0);
    assert_eq!(at_least_one(30, 0, 7), 0.0);
    assert_eq!(at_least_one(10, 5, 6), 1.0);
    // 1 - C(20,7)/C(30,7)
    let want = 1.0 - (0..7).map(|i| (20 - i) as f64 / (30 - i) as f64).product::<f64>();
    assert!((at_least_one(30, 10, 7) - want).abs() < 1e-12);
}
