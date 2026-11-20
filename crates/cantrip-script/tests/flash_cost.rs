//! "You may cast this spell as though it had flash if you pay {2} more to cast it" (CR 307.5, 601.2b, 118.8, 202.3), played on
//! a table of test-only cards until the printed ones (Harbinger of the Tides, Mystical Tether, Asinine Antics) have scenarios of
//! their own:
//!
//! - it is on offer whenever an instant could be cast, and its cost is owed only where nothing else lets the cast be made:
//!   at a time a sorcery could be cast the cast is asked whether to pay it (the rulings: "even if it's night"), and nowhere else
//!   is anything asked;
//! - it is part of the total cost, so a cast that could not pay the {2} is not on offer, and a cost reduction and the {2} combine
//!   (CR 601.2f);
//! - it is no part of the mana value (CR 202.3);
//! - a card that has flash anyway, or is an instant, never owes it.

use std::sync::OnceLock;

use cantrip_engine::*;
use cantrip_state::{Frame, FrameKind};

const COST: Cost = Cost::mana(ManaCost { generic: 2, ..ManaCost::ZERO });

const LABELS: &[&str] = &["Cast without flash", "Flash: Pay {2} more"];

/// A {1} creature with the permission.
static PROBE: CardDef = CardDef {
    name: "Flash Cost Probe",
    cost: ManaCost { generic: 1, ..ManaCost::ZERO },
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    optional_cost: Some(OptionalCost { flash: true, ..OptionalCost::new(COST) }),
    labels: LABELS,
    ..CardDef::BLANK
};

/// The same with flash of its own: it never owes the {2}, and may pay it anyway.
static FLASHER: CardDef = CardDef { name: "Flash Cost Flasher", keywords: Keywords::of(&[Keyword::Flash]), ..PROBE };


static SOURCE: CardDef = CardDef { name: "Flash Cost Source", has_cost: false, types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };

static FILLER: CardDef = CardDef { name: "Flash Cost Filler", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&PROBE, &FLASHER, &SOURCE, &FILLER]))
}

const YOU: PlayerId = PlayerId(0);
const THEY: PlayerId = PlayerId(1);

/// A game at priority in `step` of `active`'s turn with nothing on the stack, `YOU` holding `hand` and `mana` colorless mana in
/// their pool.
fn at(active: PlayerId, step: Step, hand: &[&'static CardDef], mana: u32) -> Game {
    let table = table();
    let deck = vec![table.id_of(&FILLER); 20];
    let mut game = Game::new(table, &Setup { decks: vec![deck.clone(), deck], ..Default::default() });
    game.verify = true;
    while !game.state.agenda.is_empty() {
        game.state.frame_remove(0);
    }
    game.state.update_turn(|t| {
        t.number = 3;
        t.active = active;
        t.priority = YOU;
        t.set_step(step);
    });
    game.state.update_player(active, |p| p.turn_began = 3);
    game.state.frame_push(Frame::new(FrameKind::Priority));
    for card in hand {
        let id = game.state.push_obj(Obj::new(table.id_of(card), YOU, Zone::hand(YOU)));
        game.state.zone_push(Zone::hand(YOU), id);
    }
    let source = game.state.push_obj(Obj::new(table.id_of(&SOURCE), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, source);
    game.state.add_mana(YOU, Mana::C, mana, source, None, ManaLasts::Step, false);
    game.state.see_statics();
    game.run();
    game
}

/// The cast of `name` on offer to the player holding priority, if any.
fn cast_on_offer(game: &Game, name: &str) -> Option<ActionOption> {
    let request = game.pending()?;
    let RequestKind::Priority { options } = &request.kind else { panic!("{request:?}") };
    options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == name)).cloned()
}

/// What casting asked, as `cast` saw it: the answers on offer to each question of the optional cost.
#[derive(Debug, Default)]
struct Asked {
    optional: Vec<Vec<u8>>,
}

/// Casts `name`, paying the optional cost when asked if `pay` says so, and pays for it from the pool; passes until the stack is
/// empty again. What it was asked.
fn cast(game: &mut Game, name: &str, pay: bool) -> Asked {
    let mut asked = Asked::default();
    let option = cast_on_offer(game, name).unwrap_or_else(|| panic!("{name} is not on offer"));
    let request = game.pending().cloned().unwrap();
    game.answer(request.id, YOU, Answer::Act(Action::of(&option))).unwrap();
    for _ in 0..50 {
        game.run();
        let request = game.pending().cloned().unwrap();
        let answer = match &request.kind {
            RequestKind::ChooseModes { eligible, additional_cost: true, .. } => {
                asked.optional.push(eligible.clone());
                Answer::Modes(vec![u8::from(pay && eligible.contains(&1))])
            }
            RequestKind::PayCosts { options, .. } => {
                let pay = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).or_else(|| options.iter().find(|o| o.kind == ActionKind::AssignMana));
                Answer::Act(Action::of(pay.expect("the pool pays")))
            }
            RequestKind::Priority { .. } => return asked,
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the cast did not finish: {asked:?}");
}

fn pool(game: &Game) -> u32 {
    u32::from(game.state.player(YOU).pool.get(Mana::C))
}

/// At a time a sorcery could be cast, the {2} is the caster's to pay or not.
#[test]
fn at_sorcery_speed_the_extra_cost_may_be_paid_or_not() {
    let mut game = at(YOU, Step::PrecombatMain, &[&PROBE], 3);
    let asked = cast(&mut game, "Flash Cost Probe", false);
    assert_eq!(asked.optional, vec![vec![0, 1]]);
    assert_eq!(pool(&game), 2, "only its {{1}}");

    let mut game = at(YOU, Step::PrecombatMain, &[&PROBE], 3);
    let asked = cast(&mut game, "Flash Cost Probe", true);
    assert_eq!(asked.optional, vec![vec![0, 1]]);
    assert_eq!(pool(&game), 0, "its {{1}} and the {{2}}, though it could be cast without");
}

/// On another player's turn nothing but the permission lets it be cast: it is owed, and no question is asked.
#[test]
fn at_instant_speed_the_extra_cost_is_owed() {
    let mut game = at(THEY, Step::PrecombatMain, &[&PROBE], 3);
    let asked = cast(&mut game, "Flash Cost Probe", false);
    assert!(asked.optional.is_empty(), "nothing is asked: the {{2}} is paid");
    assert_eq!(pool(&game), 0, "its {{1}} and the {{2}}, whatever the caster would answer");
}

/// It is on offer only where the {2} could be paid too.
#[test]
fn at_instant_speed_it_is_not_offered_for_its_cost_alone() {
    let game = at(THEY, Step::PrecombatMain, &[&PROBE], 2);
    assert!(cast_on_offer(&game, "Flash Cost Probe").is_none(), "{{1}} is in the pool but {{3}} is owed");
    let game = at(THEY, Step::PrecombatMain, &[&PROBE], 3);
    assert!(cast_on_offer(&game, "Flash Cost Probe").is_some());
    // At sorcery speed, the cost alone is enough.
    let game = at(YOU, Step::PrecombatMain, &[&PROBE], 1);
    assert!(cast_on_offer(&game, "Flash Cost Probe").is_some());
}

/// With something on the stack, even on its caster's own turn, a sorcery could not be cast: the {2} is owed.
#[test]
fn with_a_spell_on_the_stack_the_extra_cost_is_owed_on_its_casters_own_turn() {
    let mut game = at(YOU, Step::PrecombatMain, &[&PROBE, &PROBE], 3);
    let asked = cast(&mut game, "Flash Cost Probe", false);
    assert_eq!(asked.optional, vec![vec![0, 1]]);
    assert_eq!(game.state.stack.len(), 1);
    assert_eq!(pool(&game), 2, "the first, for {{1}}");
    assert!(cast_on_offer(&game, "Flash Cost Probe").is_none(), "{{2}} remains, {{3}} is owed");

    let mut game = at(YOU, Step::PrecombatMain, &[&PROBE, &PROBE], 4);
    cast(&mut game, "Flash Cost Probe", false);
    let asked = cast(&mut game, "Flash Cost Probe", false);
    assert!(asked.optional.is_empty(), "with a spell on the stack the {{2}} is paid, not asked");
    assert_eq!(pool(&game), 0);
}

/// A card that has flash of its own never owes it.
#[test]
fn a_card_with_flash_never_owes_the_extra_cost() {
    let mut game = at(THEY, Step::PrecombatMain, &[&FLASHER], 1);
    let asked = cast(&mut game, "Flash Cost Flasher", false);
    assert_eq!(asked.optional, vec![vec![0, 1]], "it may pay it, as any optional cost");
    assert_eq!(pool(&game), 0);
}

/// The {2} is no part of the mana value (CR 202.3).
#[test]
fn the_extra_cost_is_not_part_of_the_mana_value() {
    let mut game = at(THEY, Step::PrecombatMain, &[&PROBE], 3);
    cast(&mut game, "Flash Cost Probe", false);
    let spell = game.state.stack.last().expect("on the stack").object;
    assert_eq!(game.mana_value(spell), 1);
}
