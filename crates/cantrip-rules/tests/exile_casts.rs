//! Casting from among the cards an ability exiled (CR 607.2a, 601.3): what an activated ability's own cost exiles is exiled
//! with it, and a spell cast by an effect can be put on the bottom of its owner's library in place of the graveyard (CR
//! 614.1a); a land card in a graveyard may permit itself to be played from there (CR 113.6e, 305.1). The cards here are small probes for the machinery, not card implementations.

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

/// "You may play this card from your graveyard." A land that permits itself, as Oscorp Industries' mayhem does.
static LAND_PROBE: CardDef = CardDef {
    name: "Exile Cast Land Probe",
    oracle: true,
    types: TypeSet::of(&[CardType::Land]),
    exile_casts: &[ExileCast {
        text: "Play from your graveyard",
        from: ZoneKind::Graveyard,
        allows: |_, asker, card, player| card == asker.source && player == asker.controller,
        additional: Cost::NONE,
        any_type: false,
        any_color: false,
        lands: true,
        life_for_mana_value: false,
        instead: None,
        spell_replacement: None,
    }],
    ..CardDef::BLANK
};

static FILLER: CardDef = CardDef { name: "Exile Cast Filler", oracle: true, ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&EVIDENCE_PROBE, &FODDER, &FILLER, &LAND_PROBE]))
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

/// The first card of its controller's graveyard, cast for nothing with "if that spell would be put into a graveyard, put it on
/// the bottom of its owner's library instead", then sent off the stack as `then` says (`None`: left to resolve; a counter
/// when `counter`).
fn cast_to_bottom(g: &mut dyn cantrip_api::Game, item: &StackItem, then: Option<CounterTo>, counter: bool) -> Result<(), Suspend> {
    let Some(&card) = g.state().zone(Zone::graveyard(item.controller)).first() else { return Ok(()) };
    g.cast_during_resolution_as(card, CastTerms { instead: Some(ManaCost::ZERO), exile_instead: ExileInstead::BottomOfLibrary, ..CastTerms::default() })?;
    let Some(spell) = g.state().stack.last().map(|i| i.object).filter(|&s| s != item.object) else { return Ok(()) };
    if counter {
        g.counter(spell);
    } else if let Some(to) = then {
        g.remove_spell(spell, to);
    }
    Ok(())
}

fn cast_and_resolve(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    cast_to_bottom(g, item, None, false)
}

fn cast_and_counter(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    cast_to_bottom(g, item, None, true)
}

fn cast_and_bounce(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    cast_to_bottom(g, item, Some(CounterTo::Hand), false)
}

macro_rules! stager {
    ($name:literal, $effect:ident) => {
        CardDef { name: $name, oracle: true, types: TypeSet::of(&[CardType::Instant]), spell: Some(SpellDef { targets: &[], effect: $effect }), ..CardDef::BLANK }
    };
}

static RESOLVER: CardDef = stager!("Exile Cast Resolver", cast_and_resolve);
static COUNTERER: CardDef = stager!("Exile Cast Counterer", cast_and_counter);
static BOUNCER: CardDef = stager!("Exile Cast Bouncer", cast_and_bounce);

/// A game in which `stager` is in the first player's hand and `Exile Cast Fodder` the only card of their graveyard, with the
/// stager cast and the stack played out: the fodder's place afterward is what is judged.
fn cast_the_fodder_by(stager: &'static CardDef) -> RulesGame {
    let cards: &'static CardTable = Box::leak(Box::new(CardTable::new([&EVIDENCE_PROBE, &FODDER, &FILLER, &LAND_PROBE, stager])));
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
    let spell = put(&mut game, stager.name, Zone::hand(ME));
    put(&mut game, "Exile Cast Fodder", Zone::graveyard(ME));
    game.run();
    let RequestKind::Priority { options } = request(&game).kind else { panic!("expected priority") };
    let cast = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(spell)).expect("the stager is castable").clone();
    answer(&mut game, Answer::Act(Action::of(&cast)));
    for _ in 0..16 {
        if game.state.stack.is_empty() && game.log.iter().any(|r| matches!(r.event, Event::SpellCast { player: ME, .. })) {
            return game;
        }
        let answer_to = match request(&game).kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::PayCosts { options, .. } => {
                let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).or_else(|| options.iter().find(|o| o.kind == ActionKind::AssignMana)).expect("a payment option");
                Answer::Act(Action::of(confirm))
            }
            other => panic!("unexpected request: {other:?}"),
        };
        answer(&mut game, answer_to);
    }
    panic!("the stack did not empty");
}

/// The cards of the first player's library, top first, named.
fn library(game: &RulesGame) -> Vec<&'static str> {
    game.state.zone(Zone::library(ME)).iter().map(|&id| game.cards.get(game.state.obj(id).card).name).collect()
}

/// Kylox's Voltstrider: "if that spell would be put into a graveyard, put it on the bottom of its owner's library instead"
/// (CR 614.1a, 401.4). Resolving, the spell goes beneath the cards of the library, and not on top.
#[test]
fn a_spell_that_would_resolve_into_the_graveyard_goes_to_the_bottom_of_its_owners_library() {
    let game = cast_the_fodder_by(&RESOLVER);
    let library = library(&game);
    assert_eq!(library.last(), Some(&"Exile Cast Fodder"), "on the bottom");
    assert_eq!(library.iter().filter(|&&n| n == "Exile Cast Fodder").count(), 1);
    assert!(game.state.zone(Zone::graveyard(ME)).iter().all(|&id| game.cards.get(game.state.obj(id).card).name != "Exile Cast Fodder"), "not in the graveyard");
}

/// It replaces the move to the graveyard, however the spell comes to make it: a spell that is countered goes there too.
#[test]
fn a_countered_spell_goes_to_the_bottom_too() {
    let game = cast_the_fodder_by(&COUNTERER);
    assert_eq!(library(&game).last(), Some(&"Exile Cast Fodder"));
}

/// Only the move to the graveyard is replaced: a spell returned to its owner's hand goes there.
#[test]
fn a_spell_returned_to_hand_is_not_put_on_the_bottom() {
    let game = cast_the_fodder_by(&BOUNCER);
    assert!(!library(&game).contains(&"Exile Cast Fodder"));
    assert!(game.state.zone(Zone::hand(ME)).iter().any(|&id| game.cards.get(game.state.obj(id).card).name == "Exile Cast Fodder"));
}

/// The land plays the first player is offered now, as the cards they are of.
fn land_plays(game: &RulesGame) -> Vec<ObjectId> {
    let RequestKind::Priority { options } = request(game).kind else { panic!("expected priority") };
    options.iter().filter(|o| o.kind == ActionKind::PlayLand).filter_map(|o| o.source).collect()
}

/// A land card whose own static ability lets it be played from its owner's graveyard (CR 113.6e, 305.1) may be played from
/// there as a land play, and it enters the battlefield (CR 305.1, 305.2).
#[test]
fn a_land_that_permits_itself_is_played_from_the_graveyard() {
    let mut game = game();
    let land = put(&mut game, "Exile Cast Land Probe", Zone::graveyard(ME));
    game.run();
    assert_eq!(land_plays(&game), vec![land], "offered from the graveyard");
    let RequestKind::Priority { options } = request(&game).kind else { panic!("expected priority") };
    let play = options.iter().find(|o| o.kind == ActionKind::PlayLand).unwrap().clone();
    answer(&mut game, Answer::Act(Action::of(&play)));
    assert!(game.state.battlefield().iter().any(|&id| game.cards.get(game.state.obj(id).card).name == "Exile Cast Land Probe"), "on the battlefield");
    assert!(game.state.zone(Zone::graveyard(ME)).is_empty());
    assert_eq!(game.state.player(ME).lands_played, 1, "it was the land play of the turn");
}

/// It is a land play like another (CR 305.2): with the turn's land play used, it is not offered.
#[test]
fn a_land_played_from_the_graveyard_uses_the_land_play_of_the_turn() {
    let mut game = game();
    put(&mut game, "Exile Cast Land Probe", Zone::graveyard(ME));
    game.state.update_player(ME, |p| p.lands_played = 1);
    game.run();
    assert!(land_plays(&game).is_empty());
}
