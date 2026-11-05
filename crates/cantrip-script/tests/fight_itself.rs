//! A creature that fights itself (CR 701.14c): "If a creature fights itself, it deals damage to itself equal to twice its
//! power." No printed card in the engine can name one creature for both fighters (each says "another", or "you control" and
//! "you don't control"), so the path is played here on a table of test-only cards: a spell whose two instances of "target"
//! may name the same creature (CR 115.3), cast with both at one creature.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

fn creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.chars(id).is(CardType::Creature) && q.obj(id).zone.is(ZoneKind::Battlefield))
}

/// Target creature fights target creature.
fn fight(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let (Target::Object(a), Target::Object(b), 2) = (item.target(0), item.target(1), item.target_count) {
        g.fight(a, b);
    }
    Ok(())
}

static TWO: [TargetSpec; 2] = [target(creature), target(creature)];

static FIGHT: CardDef = CardDef { name: "Probe Fight", types: TypeSet::of(&[CardType::Instant]), spell: spell(&TWO, fight), ..CardDef::BLANK };

/// A 3/7: it survives six damage, so what was marked on it can be read.
static BRUTE: CardDef =
    CardDef { name: "Probe Brute", types: TypeSet::of(&[CardType::Creature]), power: Some(3), toughness: Some(7), ..CardDef::BLANK };

static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&FIGHT, &BRUTE, &FILLER]))
}

/// The first seat holds a Filler and a Probe Fight and has `brutes` Probe Brutes on the battlefield.
fn deal(brutes: usize) -> (Game, Vec<ObjectId>) {
    let filler = table().find("Filler").unwrap();
    let mut deck = vec![filler, table().find("Probe Fight").unwrap()];
    deck.extend(std::iter::repeat(filler).take(28));
    let mut game = Game::new(table(), &Setup { decks: vec![deck, vec![filler; 30]], seed: 1, shuffle: false, ..Setup::default() });
    let brute = table().find("Probe Brute").unwrap();
    let ids = (0..brutes)
        .map(|_| {
            let id = game.state.push_obj(Obj::new(brute, PlayerId(0), Zone::BATTLEFIELD));
            game.state.zone_push(Zone::BATTLEFIELD, id);
            id
        })
        .collect();
    game.verify = true;
    game.run();
    (game, ids)
}

/// The first seat casts Probe Fight at `first` and `second` (each instance of "target" asked in turn, CR 601.2c), and it
/// resolves.
fn cast_fight(game: &mut Game, first: ObjectId, second: ObjectId) {
    let mut cast = false;
    let mut chosen = vec![first, second].into_iter();
    for _ in 0..100 {
        game.run();
        if cast && game.state.stack.is_empty() {
            return;
        }
        let Some(request) = game.pending().cloned() else { panic!("the game ended") };
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) && !cast => {
                match options.iter().find(|o| o.kind == ActionKind::CastSpell) {
                    Some(option) => {
                        cast = true;
                        Answer::Act(Action::of(option))
                    }
                    None => Answer::Pass,
                }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseTarget { candidates, .. } => {
                let target = Target::Object(chosen.next().expect("two targets"));
                assert!(candidates.contains(&target), "{target:?} is not among {candidates:?}");
                Answer::Targets(vec![target])
            }
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("Probe Fight did not resolve");
}

/// CR 701.14c: a 3/7 that fights itself is dealt 6 damage, by itself, once; it survives.
#[test]
fn a_creature_that_fights_itself_deals_itself_twice_its_power() {
    let (mut game, brutes) = deal(1);
    cast_fight(&mut game, brutes[0], brutes[0]);
    assert!(game.state.obj(brutes[0]).zone.is(ZoneKind::Battlefield));
    assert_eq!(game.state.obj(brutes[0]).damage, 6);
    assert_eq!(damage_to(&game, brutes[0]), vec![6], "one damage event of twice its power");
}

/// Beside it, two creatures fighting each deal their power to the other (CR 701.14a): 3 each.
#[test]
fn two_creatures_that_fight_deal_each_other_their_power() {
    let (mut game, brutes) = deal(2);
    cast_fight(&mut game, brutes[0], brutes[1]);
    assert_eq!(game.state.obj(brutes[0]).damage, 3);
    assert_eq!(game.state.obj(brutes[1]).damage, 3);
    assert_eq!(damage_to(&game, brutes[0]), vec![3]);
}

/// The amounts of each damage event dealt to `id`, in order.
fn damage_to(game: &Game, id: ObjectId) -> Vec<i64> {
    game.history()
        .all()
        .iter()
        .filter_map(|r| match r.event {
            Event::Damage { target, amount, .. } if target == Target::Object(id) => Some(amount),
            _ => None,
        })
        .collect()
}
