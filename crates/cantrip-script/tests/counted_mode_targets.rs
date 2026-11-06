//! A modal triggered ability's target slots (CR 700.2, 603.3d, 601.2c): a mode's required slot is filled without
//! asking only when its fewest takes every candidate there is, and an X slot takes X - so with X 1 and two creatures, one is
//! chosen, not both, and "one or two" of two creatures is still asked. The X is the source's, cast with it (CR 107.3m). No printed card in the engine has a modal triggered ability with
//! X targets, so a test card does here.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

fn creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

fn this_enters(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. } if *new == source && to.is(ZoneKind::Battlefield) => Some(Fired { subject: Target::None, data: [0, 0] }),
        _ => None,
    }
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

fn gain_one(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

static MODES: ModalDef = modes(&[
    mode(tap_them, "Tap X target creatures.").targets(&[target(creature).counted()]),
    mode(gain_one, "You gain 1 life."),
]);

static RANGE_MODES: ModalDef = modes(&[
    mode(tap_them, "Tap one or two target creatures.").targets(&[target(creature).between(1, 2)]),
    mode(gain_one, "You gain 1 life."),
]);

/// An {X} 1/1: "When this creature enters, choose one - Tap X target creatures; or you gain 1 life."
static RANGER: CardDef = CardDef {
    name: "Probe Ranger",
    types: TypeSet::of(&[CardType::Creature]),
    cost: ManaCost { x: 1, ..ManaCost::ZERO },
    power: Some(1),
    toughness: Some(1),
    abilities: &[modal_triggered(when(EventMask::ZONE_CHANGE, this_enters), &MODES, "When this creature enters, choose one \u{2014} Tap X target creatures; or you gain 1 life.")],
    ..CardDef::BLANK
};

/// An {X} 1/1: "When this creature enters, choose one - Tap one or two target creatures; or you gain 1 life."
static WARDEN: CardDef = CardDef {
    name: "Probe Warden",
    types: TypeSet::of(&[CardType::Creature]),
    cost: ManaCost { x: 1, ..ManaCost::ZERO },
    power: Some(1),
    toughness: Some(1),
    abilities: &[modal_triggered(when(EventMask::ZONE_CHANGE, this_enters), &RANGE_MODES, "When this creature enters, choose one \u{2014} Tap one or two target creatures; or you gain 1 life.")],
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
    TABLE.get_or_init(|| CardTable::new([&RANGER, &WARDEN, &BEAR, &SPRING, &FILLER]))
}

/// Bot1 casts `card` with X 1, two of Bot2's Bears on the battlefield, chooses its first mode and the first target it is
/// offered. How many targets it was asked for at most (None if it was not asked), and how many Bears end up tapped.
fn play(card: &str) -> (Option<u32>, usize) {
    let names = |list: &[&str]| -> Vec<CardId> { list.iter().map(|n| table().find(n).unwrap_or_else(|| panic!("no card {n}"))).collect() };
    let mut first = names(&[card]);
    first.extend(names(&["Filler"; 29]));
    let mut game = Game::new(table(), &Setup { decks: vec![first, names(&["Filler"; 30])], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let put = |game: &mut Game, name: &str, owner: PlayerId| {
        let id = game.state.push_obj(Obj::new(table().find(name).unwrap(), owner, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
        id
    };
    let spring = put(&mut game, "Probe Spring", PlayerId(0));
    put(&mut game, "Probe Bear", PlayerId(1));
    put(&mut game, "Probe Bear", PlayerId(1));
    let (mut cast, mut asked) = (false, None);
    for _ in 0..500 {
        game.run();
        let settled = game.state.stack.is_empty() && game.state.triggers.is_empty() && matches!(game.pending().map(|r| &r.kind), Some(RequestKind::Priority { .. }));
        if cast && settled {
            break;
        }
        let request = game.pending().cloned().expect("the game goes on");
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) && !cast && game.state.turn.step() == Step::PrecombatMain => {
                cast = true;
                let option = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == card));
                Answer::Act(Action::of(option.expect("the card is offered")))
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::PayCosts { announcing: true, x, options, .. } => {
                let kind = if *x < 1 { ActionKind::IncreaseX } else { ActionKind::ConfirmCasting };
                Answer::Act(Action::of(options.iter().find(|o| o.kind == kind).unwrap_or_else(|| panic!("no {kind:?} at X {x}"))))
            }
            RequestKind::PayCosts { options, .. } => {
                let option = options
                    .iter()
                    .find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(spring))
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::AssignMana))
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::ConfirmCasting));
                Answer::Act(Action::of(option.unwrap_or_else(|| panic!("the mana to pay with: {request:?}"))))
            }
            RequestKind::ChooseModes { .. } => Answer::Modes(vec![0]),
            RequestKind::ChooseTarget { candidates, count, .. } => {
                asked = Some(*count);
                Answer::Targets(vec![candidates[0]])
            }
            RequestKind::OrderTriggers { sources, .. } => Answer::Order((0..count_u32(sources.len())).collect()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert!(cast, "the card was not cast");
    let tapped = game.state.battlefield().iter().filter(|&&id| game.name(id) == "Probe Bear" && game.state.obj(id).tapped).count();
    (asked, tapped)
}

#[test]
fn x_targets_of_a_mode_with_more_candidates_than_x_are_chosen() {
    assert_eq!(play("Probe Ranger"), (Some(1), 1));
}

/// "One or two target creatures" with two candidates is still a choice (CR 601.2c): the player is asked, and may take one.
#[test]
fn one_or_two_targets_of_a_mode_with_two_candidates_are_asked_for() {
    assert_eq!(play("Probe Warden"), (Some(2), 1));
}
