//! `Game::fork_for_search` with a question outstanding that the game is part of the way through: a fork answers it one
//! way, goes back to its root and answers it another, as the real game would, without reaching into the real game's
//! journal or tree. The positions are scenarios handed over at the question.

use cantrip_engine::*;
use cantrip_script::{GameScript, run};

/// The game a scenario hands over at its question, checked as it plays.
fn handed_over(text: &str) -> Game {
    let script = GameScript::parse(text, "fork").expect("parses");
    let result = run(&script).map_err(|f| f.report).expect("reaches its handover");
    assert!(result.handed_over);
    let mut game = result.game;
    game.verify = true;
    game
}

fn name(game: &Game, id: ObjectId) -> &'static str {
    game.cards.get(game.state.obj(id).card).name
}

fn names(game: &Game, zone: Zone) -> Vec<&'static str> {
    game.state.zone(zone).iter().map(|&id| name(game, id)).collect()
}

/// Duress resolving, Bot1 to choose which of Bot2's cards is discarded.
const DURESS_CHOOSES: &str = "
[Bot1 Deck]
Swamp
Duress
Swamp x10

[Bot2 Deck]
Lightning Bolt
Counterspell
Guerrilla Gorilla
Island x10

[Actions]
Bot1.Hand.Swamp.Play
Bot1.Battlefield.Swamp.Activate
Bot1.Hand.Duress.Cast(Bot2)
[Handover Bot1.Choose]
";

/// Duress cast, its mana not yet paid: Bot1 is asked to pay.
const DURESS_PAYS: &str = "
[Bot1 Deck]
Swamp
Duress
Swamp x10

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Swamp.Play
Bot1.Hand.Duress.Cast(Bot2)
[Handover Bot1.PayCost]
";

/// Chooses the eligible card named `card`.
fn choose(game: &mut Game, card: &str) {
    let request = game.pending().cloned().expect("a question");
    let RequestKind::ChooseCards { eligible, .. } = &request.kind else { panic!("a card choice: {request:?}") };
    let id = eligible.iter().copied().find(|&id| name(game, id) == card).expect("eligible");
    game.answer(request.id, request.player, Answer::Cards(vec![id])).expect("legal");
}

/// A fork answers a resolution's question one way, then from its root another: each is the position the real game
/// reaches with that answer, and the real game is untouched.
#[test]
fn a_fork_answers_a_question_asked_mid_resolution_one_way_then_another() {
    let game = handed_over(DURESS_CHOOSES);
    let (state, log, nodes) = (game.state.clone(), game.log.clone(), game.tree.len());
    let bot2 = PlayerId(1);

    let mut fork = game.fork_for_search();
    let root = fork.checkpoint();
    choose(&mut fork, "Lightning Bolt");
    assert_eq!(names(&fork, Zone::graveyard(bot2)), ["Lightning Bolt"]);
    assert!(fork.state.stack.is_empty());
    let bolt = fork.state.clone();

    fork.goto(root);
    assert!(fork.state == state && fork.log == log);
    choose(&mut fork, "Counterspell");
    assert_eq!(names(&fork, Zone::graveyard(bot2)), ["Counterspell"]);
    assert!(fork.state.zone(Zone::hand(bot2)).iter().any(|&id| name(&fork, id) == "Lightning Bolt"));
    let counterspell = (fork.state.clone(), fork.log.clone());

    fork.goto(root);
    choose(&mut fork, "Lightning Bolt");
    assert!(fork.state == bolt, "the first answer's node, gone back down to");

    assert!(game.state == state && game.log == log && game.tree.len() == nodes, "the real game is untouched");
    let mut real = game;
    choose(&mut real, "Counterspell");
    assert!(real.state == counterspell.0 && real.log == counterspell.1, "the real game reaches the fork's position");
}

/// Pays Bot1's spell with the first mana ability offered until it can be confirmed, and confirms it.
fn pay(game: &mut Game) {
    loop {
        let request = game.pending().cloned().expect("a question");
        let RequestKind::PayCosts { options, .. } = &request.kind else { return };
        let option = options
            .iter()
            .find(|o| o.kind == ActionKind::ConfirmCasting)
            .or_else(|| options.iter().find(|o| o.kind == ActionKind::ActivateAbility))
            .expect("a way to pay");
        game.answer(request.id, request.player, Answer::Act(Action::of(option))).expect("legal");
    }
}

/// A fork pays for a spell being cast, then from its root cancels the cast instead (CR 733.1): the spell is back in hand
/// and the land untapped, as when the real game cancels; then pays again and comes to the same position.
#[test]
fn a_fork_pays_for_a_spell_being_cast_then_cancels_it_instead() {
    let game = handed_over(DURESS_PAYS);
    let (state, log, nodes) = (game.state.clone(), game.log.clone(), game.tree.len());
    let bot1 = PlayerId(0);
    let swamp = |g: &Game| g.state.battlefield().iter().copied().find(|&id| name(g, id) == "Swamp").expect("the Swamp");

    let mut fork = game.fork_for_search();
    let root = fork.checkpoint();
    pay(&mut fork);
    assert!(matches!(fork.pending().map(|r| &r.kind), Some(RequestKind::Priority { .. })));
    assert_eq!(fork.state.stack.len(), 1);
    assert!(fork.state.obj(swamp(&fork)).tapped);
    let paid = (fork.state.clone(), fork.log.clone());

    fork.goto(root);
    assert!(fork.state == state && fork.log == log);
    let request = fork.pending().cloned().expect("the payment");
    fork.answer(request.id, request.player, Answer::Cancel).expect("legal");
    assert!(fork.state.stack.is_empty());
    assert!(names(&fork, Zone::hand(bot1)).contains(&"Duress"));
    assert!(!fork.state.obj(swamp(&fork)).tapped);
    let cancelled = (fork.state.clone(), fork.log.clone());

    fork.goto(root);
    pay(&mut fork);
    assert!(fork.state == paid.0 && fork.log == paid.1);

    assert!(game.state == state && game.log == log && game.tree.len() == nodes, "the real game is untouched");
    let mut real = game.clone();
    real.answer(request.id, request.player, Answer::Cancel).expect("legal");
    assert!(real.state == cancelled.0 && real.log == cancelled.1, "the real game cancels to the fork's position");
    let mut real = game;
    pay(&mut real);
    assert!(real.state == paid.0 && real.log == paid.1, "the real game pays to the fork's position");
}
