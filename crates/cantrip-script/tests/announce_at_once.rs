//! X announced in one answer (CR 601.2b): `Answer::Announce` takes any value up to the announcement's `most` and is refused
//! past it, the question left outstanding; 2 announced for "tap X target creatures" taps two. A test card does it, since the
//! answer is the rules' and no card's own.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

fn creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

/// Tap each of its targets still legal.
fn tap_them(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    for slot in 0..item.target_count as usize {
        if g.target_legal(item, slot)
            && let Target::Object(id) = g.state().target(item, slot)
        {
            g.tap(id);
        }
    }
    Ok(())
}

/// {X} sorcery: "Tap X target creatures."
static SWEEP: CardDef = CardDef {
    name: "Probe Sweep",
    types: TypeSet::of(&[CardType::Sorcery]),
    cost: ManaCost { x: 1, ..ManaCost::ZERO },
    spell: spell(&[target(creature).counted()], tap_them),
    ..CardDef::BLANK
};

static BEAR: CardDef = CardDef {
    name: "Probe Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// A land: "{T}: Add {C}."
static SPRING: CardDef = CardDef {
    name: "Probe Spring",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    abilities: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}.")],
    ..CardDef::BLANK
};

static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&SWEEP, &BEAR, &SPRING, &FILLER]))
}

#[test]
fn x_is_announced_at_once_up_to_the_most_and_refused_past_it() {
    let names = |list: &[&str]| -> Vec<CardId> { list.iter().map(|n| table().find(n).unwrap_or_else(|| panic!("no card {n}"))).collect() };
    let mut first = names(&["Probe Sweep"]);
    first.extend(names(&["Filler"; 29]));
    let mut game = Game::new(table(), &Setup { decks: vec![first, names(&["Filler"; 30])], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let put = |game: &mut Game, name: &str, owner: PlayerId| {
        let id = game.state.push_obj(Obj::new(table().find(name).unwrap(), owner, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
        id
    };
    put(&mut game, "Probe Spring", PlayerId(0));
    put(&mut game, "Probe Spring", PlayerId(0));
    let bears = [put(&mut game, "Probe Bear", PlayerId(1)), put(&mut game, "Probe Bear", PlayerId(1))];
    let (mut cast, mut announced) = (false, None);
    for _ in 0..500 {
        game.run();
        let settled = game.state.stack.is_empty() && matches!(game.pending().map(|r| &r.kind), Some(RequestKind::Priority { .. }));
        if announced.is_some() && settled {
            break;
        }
        let request = game.pending().cloned().expect("the game goes on");
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) && !cast && game.state.turn.step() == Step::PrecombatMain => {
                cast = true;
                Answer::Act(Action::of(options.iter().find(|o| o.kind == ActionKind::CastSpell).expect("the sweep is offered")))
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::PayCosts { announcing: true, most, .. } => {
                assert!(*most >= 2);
                assert!(game.answer(request.id, request.player, Answer::Announce(*most + 1)).is_err(), "past the most is refused");
                assert_eq!(game.pending().map(|r| r.id), Some(request.id), "the refused answer leaves the question outstanding");
                announced = Some(2);
                Answer::Announce(2)
            }
            RequestKind::PayCosts { options, .. } => Answer::Act(Action::of(
                options
                    .iter()
                    .find(|o| o.kind == ActionKind::ActivateAbility)
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::AssignMana))
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::ConfirmCasting))
                    .unwrap_or_else(|| panic!("the mana to pay with: {request:?}")),
            )),
            RequestKind::ChooseTarget { candidates, count, .. } => Answer::Targets(candidates[..*count as usize].to_vec()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert_eq!(announced, Some(2));
    assert!(bears.iter().all(|&bear| game.state.obj(bear).tapped), "X 2 tapped both Bears");
}
