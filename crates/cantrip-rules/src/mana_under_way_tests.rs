//! A mana ability part way through its activation is part of the position (`Frame::mana_abilities`, CR 605.3a, 602.2): what is
//! offered at a question asked while it is paid, and what the rules are asked of a copy of that position, stay exact as the
//! game-log tree moves away from it and back, and are gone at a position where nothing is being activated.
use crate::actions::payment_mana_options_with_mana_to_spare;
use crate::{Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

static PLAIN: CardDef = CardDef {
    name: "Under Way Plain Mana", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}.")], ..CardDef::BLANK
};
// Its {T} is in the cost being paid while its {1} is: it is never offered to pay itself.
static PAYING: CardDef = CardDef {
    name: "Under Way Paying Mana",
    abilities: &[mana_ability(TAP.and(cost("{1}")), add(Mana::C), "{1}, {T}: Add {C}.")], ..PLAIN
};
static SPELL: CardDef = CardDef {
    name: "Under Way Spell", cost: mana("{2}"), has_cost: true, types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK
};
static FILLER: CardDef = CardDef { name: "Under Way Filler", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn put(game: &mut Game, def: &'static CardDef) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

fn offers(options: &[ActionOption], source: ObjectId) -> bool {
    options.iter().any(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(source))
}

fn to_main_phase(game: &mut Game) -> Request {
    game.verify = true;
    game.run();
    for _ in 0..40 {
        if game.state.turn.step() == Step::PrecombatMain && game.pending().is_some_and(|r| r.player == YOU) { break; }
        let request = game.pending().cloned().expect("priority");
        game.answer(request.id, request.player, Answer::Pass).expect("pass");
    }
    game.pending().cloned().expect("main phase")
}

fn activate(game: &mut Game, source: ObjectId) {
    let request = game.pending().cloned().expect("a question");
    let options = match &request.kind {
        RequestKind::Priority { options } | RequestKind::PayCosts { options, .. } => options,
        other => panic!("{other:?}"),
    };
    let option = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(source)).expect("offered");
    game.answer(request.id, YOU, Answer::Act(Action::of(option))).expect("activate");
}

#[test]
fn a_mana_ability_activated_while_a_spell_is_paid_is_no_longer_under_way_once_activated() {
    let cards = Box::leak(Box::new(CardTable::new([&PLAIN, &PAYING, &SPELL, &FILLER])));
    let mut game = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&FILLER); 20]; 2], ..Setup::default() });
    let paying = put(&mut game, &PAYING);
    let plain = put(&mut game, &PLAIN);
    put(&mut game, &PLAIN);
    let spell = game.state.push_obj(Obj::new(game.cards.id_of(&SPELL), YOU, Zone::hand(YOU)));
    game.state.zone_push(Zone::hand(YOU), spell);
    let request = to_main_phase(&mut game);
    let RequestKind::Priority { options } = &request.kind else { panic!("priority") };
    let cast = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(spell)).expect("castable");
    game.answer(request.id, YOU, Answer::Act(Action::of(cast))).expect("cast");
    // The spell's {2}: the paying source's ability, its own {1} paid by a plain source; then the spell still owes {1}.
    activate(&mut game, paying);
    activate(&mut game, plain);
    let request = game.pending().cloned().expect("the spell's payment goes on");
    let RequestKind::PayCosts { source, .. } = request.kind else { panic!("a payment") };
    assert_eq!(game.state.obj(source).card, cards.id_of(&SPELL), "the spell, on the stack");
    assert!(game.state.agenda.iter().all(|f| f.mana_abilities.is_empty()), "the paying source's ability has been activated");
}

#[test]
fn a_mana_ability_being_paid_is_under_way_at_its_question_and_after_the_tree_moves_away_and_back() {
    let cards = Box::leak(Box::new(CardTable::new([&PLAIN, &PAYING, &SPELL, &FILLER])));
    let mut game = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&FILLER); 20]; 2], ..Setup::default() });
    let paying = put(&mut game, &PAYING);
    let plain = put(&mut game, &PLAIN);
    let request = to_main_phase(&mut game);
    let RequestKind::Priority { options } = &request.kind else { panic!("priority") };
    let activate = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(paying)).expect("offered");
    let answer = Answer::Act(Action::of(activate));
    let root = game.checkpoint();
    // Nothing is being activated: a payment here could use the paying source's ability.
    assert!(offers(&payment_mana_options_with_mana_to_spare(&game, YOU), paying));

    game.answer(request.id, YOU, answer.clone()).expect("activate");
    let child = game.checkpoint();
    let paying_question = |game: &Game| {
        let request = game.pending().cloned().expect("its {1} is asked for");
        let RequestKind::PayCosts { options, source, .. } = request.kind else { panic!("a payment") };
        assert_eq!(source, paying);
        options
    };
    let exact = |game: &Game| {
        let options = paying_question(game);
        assert!(offers(&options, plain) && !offers(&options, paying));
        let advice = payment_mana_options_with_mana_to_spare(game, YOU);
        assert!(offers(&advice, plain) && !offers(&advice, paying));
    };
    exact(&game);

    // Back to before the activation: nothing is under way there.
    game.goto(root);
    assert!(game.state.agenda.iter().all(|f| f.mana_abilities.is_empty()));
    assert!(offers(&payment_mana_options_with_mana_to_spare(&game, YOU), paying));
    // And forward to the question again, by the tree and by the same answer.
    game.goto(child);
    exact(&game);
    game.goto(root);
    game.answer(request.id, YOU, answer).expect("the same answer follows the existing child");
    assert_eq!(game.checkpoint(), child);
    exact(&game);
}
