//! Tokens created attacking through `Game::create_tokens_worded` (mobilize's path): tapped only when the effect says
//! "tapped and attacking" (CR 508.4), over test cards, since no printed card in content makes worded tokens that enter
//! attacking untapped.

use crate::{Cx, Game, Setup};
use cantrip_api::*;

static SOLDIER: CardDef = CardDef {
    name: "Attacking Test Soldier",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// Player 0's turn, in combat with attackers declared, making one token attacking player 1, `tapped` as given.
fn made(tapped: bool) -> (Game, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&SOLDIER])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    game.state.update_turn(|t| {
        t.active = PlayerId(0);
        t.set_step(Step::DeclareAttackers);
        t.attacked = true;
    });
    let card = table.id_of(&SOLDIER);
    let token = {
        let mut cx = cx(&mut game);
        cantrip_api::Game::create_tokens_worded(&mut cx, &[(card, PlayerId(0))], tapped, Some(Target::Player(PlayerId(1))), Words::NONE)
            .expect("nothing is asked")[0]
    };
    (game, token)
}

#[test]
fn a_worded_token_created_attacking_is_untapped_unless_the_effect_says_tapped() {
    let (game, token) = made(false);
    assert!(!game.state.obj(token).tapped);
    assert!(game.state.attackers.iter().any(|a| a.id == token));
    let (game, token) = made(true);
    assert!(game.state.obj(token).tapped);
    assert!(game.state.attackers.iter().any(|a| a.id == token));
}

/// An ETB intervening condition as printed on Thousand-Faced Shadow. An
/// arriving token copy can have this ability as well (CR 603.6a, 508.4).
fn entered_attacking(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. } if *new == asker.source && to.is(ZoneKind::Battlefield) && q.is_attacking(*new) => Fired::about(Target::Object(*new)),
        _ => None,
    }
}
fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> { Ok(()) }
static WATCHER: CardDef = CardDef {
    name: "Attacking Arrival Watcher", types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(1),
    abilities: &[triggered(when_asked(EventMask::ZONE_CHANGE, entered_attacking), nothing, "When this enters, if it's attacking.")],
    ..CardDef::BLANK
};
static LAND: CardDef = CardDef { name: "Attacking Arrival Land", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn creature_arriving(q: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.to.is(ZoneKind::Battlefield) && q.chars(z.object).is(CardType::Creature))
}
fn arrive_attacking(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal { z.attacking = true; }
}
static FIREBRAND: CardDef = CardDef {
    name: "Attacking Arrival Replacement", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, creature_arriving, arrive_attacking, "Creatures enter attacking.")],
    ..CardDef::BLANK
};

fn arrival_game(combat: bool) -> Game {
    let table = Box::leak(Box::new(CardTable::new([&WATCHER, &SOLDIER])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    game.state.update_turn(|t| {
        t.active = PlayerId(0);
        t.set_step(if combat { Step::DeclareBlockers } else { Step::PrecombatMain });
        t.attacked = combat;
    });
    game
}

#[test]
fn attacking_state_is_visible_to_etb_for_cards_worded_tokens_and_token_copies() {
    for path in 0..4 {
        let mut game = arrival_game(true);
        let card = game.cards.id_of(&WATCHER);
        let made = {
            let mut cx = cx(&mut game);
            match path {
                0 => {
                    let hand = cx.st().push_obj(Obj::new(card, PlayerId(0), Zone::hand(PlayerId(0))));
                    cx.st().zone_push(Zone::hand(PlayerId(0)), hand);
                    crate::ops::put_onto_battlefield_attacking(&mut cx, hand, PlayerId(0), Target::Player(PlayerId(1)), &[]).unwrap()
                }
                1 => cantrip_api::Game::create_tokens_worded(&mut cx, &[(card, PlayerId(0))], true, Some(Target::Player(PlayerId(1))), Words::NONE).unwrap()[0],
                2 => cantrip_api::Game::create_tokens_attacking(&mut cx, &[(card, PlayerId(0))], true, Target::Player(PlayerId(1))).unwrap()[0],
                _ => cantrip_api::Game::create_token_copies_attacking(&mut cx, Copiable::of(card), PlayerId(0), 1, true, None).unwrap()[0],
            }
        };
        assert!(game.state.obj(made).tapped);
        let attacker = game.state.attackers.iter().find(|a| a.id == made).unwrap();
        assert_eq!(attacker.target, Target::Player(PlayerId(1)));
        assert!(game.state.triggers.iter().any(|t| t.source == made), "ETB condition must see the attacker on path {path}");
        assert!(!game.log.iter().any(|r| matches!(r.event, Event::Attacks { attacker, .. } if attacker == made)), "arrivals are never declared attackers");
    }
}

#[test]
fn an_arrival_outside_combat_or_for_the_wrong_player_is_not_attacking() {
    for (combat, controller, defender) in [
        (false, PlayerId(0), PlayerId(1)),
        (true, PlayerId(1), PlayerId(0)),
        (true, PlayerId(0), PlayerId(0)),
    ] {
        let mut game = arrival_game(combat);
        let card = game.cards.id_of(&WATCHER);
        let made = cantrip_api::Game::create_token_copies_attacking(&mut cx(&mut game), Copiable::of(card), controller, 1, true, Some(Target::Player(defender))).unwrap()[0];
        assert!(game.state.obj(made).tapped);
        assert!(!game.state.attackers.iter().any(|a| a.id == made));
        assert!(!game.state.triggers.iter().any(|t| t.source == made));
    }
}

#[test]
fn ineligible_arrivals_choose_no_defender_even_when_there_are_several() {
    for (def, combat, controller) in [(&LAND, true, PlayerId(0)), (&WATCHER, false, PlayerId(0)), (&WATCHER, true, PlayerId(1))] {
        let table = Box::leak(Box::new(CardTable::new([&LAND, &WATCHER])));
        let mut game = Game::new(table, &Setup { decks: vec![Vec::new(); 3], ..Default::default() });
        game.state.update_turn(|t| { t.active = PlayerId(0); t.set_step(Step::DeclareBlockers); t.attacked = combat; });
        // There is deliberately no resolving frame to ask from. Asking a
        // defender for a noncreature or nonattacking player would fail here.
        let created = cantrip_api::Game::create_token_copies_attacking(&mut cx(&mut game), Copiable::of(table.id_of(def)), controller, 1, true, None).unwrap();
        assert_eq!(created.len(), 1);
        assert!(game.state.attackers.is_empty());
        assert!(game.pending().is_none());
    }
}

#[test]
fn an_instruction_and_replacement_choose_a_defender_only_once() {
    for explicit in [false, true] {
        let table = Box::leak(Box::new(CardTable::new([&WATCHER, &FIREBRAND])));
        let mut game = Game::new(table, &Setup { decks: vec![Vec::new(); 3], ..Default::default() });
        game.state.update_turn(|t| { t.active = PlayerId(0); t.set_step(Step::DeclareBlockers); t.attacked = true; });
        let giver = game.state.push_obj(Obj::new(table.id_of(&FIREBRAND), PlayerId(0), Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, giver);
        game.state.see_statics();
        let mut frame = cantrip_state::Frame::new(cantrip_state::FrameKind::Priority);
        frame.answers = vec![(None, Answer::Targets(vec![Target::Player(PlayerId(2))])), (None, Answer::Targets(vec![Target::Player(PlayerId(1))]))];
        game.state.frame_push(frame);
        let frame = game.state.agenda.len() - 1;
        let mut cx = cx(&mut game);
        cx.frame = frame;
        let defender = explicit.then_some(Target::Player(PlayerId(2)));
        let created = cantrip_api::Game::create_token_copies_attacking(&mut cx, Copiable::of(table.id_of(&WATCHER)), PlayerId(0), 1, true, defender).unwrap();
        assert_eq!(cx.cursor, usize::from(!explicit), "an explicit defender needs no choice; an unspecified defender needs one");
        assert_eq!(cx.s().attackers.iter().find(|a| a.id == created[0]).unwrap().target, Target::Player(PlayerId(2)));
    }
}
