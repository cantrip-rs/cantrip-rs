//! Casting from among the cards an ability exiled (CR 607.2a, 601.3): what an activated ability's own cost exiles is exiled
//! with it, and a spell cast by an effect can be put on the bottom of its owner's library in place of the graveyard (CR
//! 614.1a). The cards here are small probes for the machinery, not card implementations.

use std::sync::OnceLock;

use cantrip_api::*;
use cantrip_rules::{Game as RulesGame, Setup};
use cantrip_state::{Frame, FrameKind};

const ME: PlayerId = PlayerId(0);

fn nothing_happens(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

fn sacrifice_this(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.sacrifice(item.controller, &[item.object]);
    Ok(())
}

/// "Collect evidence 2: Sacrifice this." The cost exiles cards from its controller's graveyard.
static EVIDENCE_PROBE: CardDef = CardDef {
    name: "Exile Cast Evidence Probe",
    oracle: true,
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost { collect_evidence: 2, ..Cost::NONE }, sacrifice_this, "Collect evidence 2: Sacrifice this.")],
    ..CardDef::BLANK
};

/// A card of mana value 2.
static FODDER: CardDef = CardDef {
    name: "Exile Cast Fodder",
    oracle: true,
    cost: ManaCost { generic: 2, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: Some(SpellDef { targets: &[], effect: nothing_happens }),
    ..CardDef::BLANK
};

static FILLER: CardDef = CardDef { name: "Exile Cast Filler", oracle: true, ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&EVIDENCE_PROBE, &FODDER, &FILLER]))
}

/// A game in the first player's precombat main phase with priority.
fn game() -> RulesGame {
    let cards = table();
    let filler = cards.find("Exile Cast Filler").unwrap();
    let mut game = RulesGame::new(cards, &Setup { decks: vec![vec![filler; 20], vec![filler; 20]], seed: 1, ..Setup::default() });
    game.state.update_turn(|t| {
        t.number = 1;
        t.active = ME;
        t.priority = ME;
        t.set_step(Step::PrecombatMain);
        t.began = true;
    });
    game.state.frame_push(Frame::new(FrameKind::Priority));
    game
}

/// A card of `name` put into `zone` of the first player.
fn put(game: &mut RulesGame, name: &str, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.find(name).unwrap(), ME, zone));
    game.state.zone_push(zone, id);
    id
}

fn request(game: &RulesGame) -> Request {
    game.pending().cloned().expect("a question")
}

fn answer(game: &mut RulesGame, answer: Answer) {
    let request = request(game);
    game.answer(request.id, request.player, answer).unwrap();
    game.run();
}

/// Activates `source`'s first ability from priority.
fn activate(game: &mut RulesGame, source: ObjectId) {
    let RequestKind::Priority { options } = request(game).kind else { panic!("expected priority") };
    let option = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(source)).expect("the ability is offered").clone();
    answer(game, Answer::Act(Action::of(&option)));
}

/// The cards in the first player's exile that were exiled with `by` (CR 607.2a), found as an ability of its own text asks.
fn exiled_with(game: &RulesGame, by: ObjectId) -> Vec<ObjectId> {
    let linked = Linked { object: by, text: game.state.obj(by).card, acquired: Acquired::OWN };
    Query::exiled_with(game, linked)
}

/// Collect evidence is an instruction to exile (CR 701.59a), so what an ability's cost exiles is exiled with the object whose
/// ability it is (CR 607.2a): a linked ability that refers to "cards exiled with it" finds them.
#[test]
fn the_cards_an_ability_collects_as_evidence_are_exiled_with_its_source() {
    let mut game = game();
    let probe = put(&mut game, "Exile Cast Evidence Probe", Zone::BATTLEFIELD);
    let fodder = put(&mut game, "Exile Cast Fodder", Zone::graveyard(ME));
    game.run();
    activate(&mut game, probe);
    match request(&game).kind {
        RequestKind::CollectEvidence { .. } => answer(&mut game, Answer::Cards(vec![fodder])),
        other => panic!("expected the evidence to collect, got {other:?}"),
    }
    let exiled = game.state.zone(Zone::exile(ME)).to_vec();
    assert_eq!(exiled.len(), 1, "the card is exiled");
    assert_eq!(exiled_with(&game, probe), exiled, "and exiled with the object whose ability it paid for");
}

/// The exile is not "until" the source leaves (CR 610.3): what the cost exiled stays exiled when the object it is exiled with
/// has left the battlefield.
#[test]
fn the_cards_collected_as_evidence_stay_exiled_when_their_source_leaves() {
    let mut game = game();
    let probe = put(&mut game, "Exile Cast Evidence Probe", Zone::BATTLEFIELD);
    let fodder = put(&mut game, "Exile Cast Fodder", Zone::graveyard(ME));
    game.run();
    activate(&mut game, probe);
    answer(&mut game, Answer::Cards(vec![fodder]));
    for _ in 0..4 {
        if game.state.stack.is_empty() {
            break;
        }
        answer(&mut game, Answer::Pass);
    }
    assert!(game.state.stack.is_empty(), "the ability resolved");
    assert!(!game.state.zone(Zone::BATTLEFIELD).contains(&probe), "and the probe was sacrificed");
    assert_eq!(game.state.zone(Zone::exile(ME)).len(), 1, "the card the cost exiled is still exiled");
}
