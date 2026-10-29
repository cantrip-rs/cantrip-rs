//! A card cast as an effect resolves by a player the effect names, not its controller (CR 608.2g, 601.2,
//! `CastTerms::caster`): "the exiled card's owner may cast that card" (Spell Queller), "that player may cast" (Knowledge
//! Pool), "that opponent may cast" (Allure of the Unknown). That player announces the spell, chooses its targets as its
//! "you", pays its costs from their own mana and controls it (CR 405.4). The cards here are probes, not card
//! implementations.

use std::sync::OnceLock;

use cantrip_api::*;
use cantrip_rules::{Game as RulesGame, Setup};
use cantrip_state::{Frame, FrameKind};

const ME: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

/// "Target opponent": an opponent of the spell's controller.
fn an_opponent(_: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Player(p) if p != asker.controller)
}

fn nothing_happens(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// A {1} instant that targets an opponent.
static AIMED: CardDef = CardDef {
    name: "Caster Aimed Instant",
    oracle: true,
    cost: ManaCost { generic: 1, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Instant]),
    spell: Some(SpellDef { targets: &[target(an_opponent)], effect: nothing_happens }),
    ..CardDef::BLANK
};

/// A {1} instant that targets a creature its caster controls ("target creature you control").
fn your_creature(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.obj(id).zone.is(ZoneKind::Battlefield) && q.chars(id).is(CardType::Creature) && q.controller(id) == asker.controller)
}

static YOURS: CardDef = CardDef {
    name: "Caster Your Creature Instant",
    spell: Some(SpellDef { targets: &[target(your_creature)], effect: nothing_happens }),
    ..AIMED
};

/// "The owner of the first card in an opponent's graveyard casts it, paying its costs": what is asked is whether the owner
/// could, and the cast is theirs (`CastTerms::caster`).
fn opponent_casts_their_card(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let them = PlayerId(1 - item.controller.0);
    let Some(&card) = g.state().zone(Zone::graveyard(them)).first() else { return Ok(()) };
    let mut checks = Vec::new();
    checks.push(g.could_cast_during_resolution_by(card, them));
    checks.push(g.could_cast_during_resolution(card));
    checks.push(!g.cast_ways_during_resolution_by(card, them).is_empty());
    // What each judged is recorded where the test reads it: a counter on nothing would change the game, so a life total.
    let gained = checks.iter().enumerate().map(|(n, &c)| if c { 1 << n } else { 0 }).sum::<i32>();
    g.gain_life(item.controller, gained);
    g.cast_during_resolution_as(card, CastTerms { caster: Some(them), ..CastTerms::default() })?;
    Ok(())
}

static CASTER_PROBE: CardDef = CardDef {
    name: "Caster Probe",
    cost: ManaCost::ZERO,
    spell: Some(SpellDef { targets: &[], effect: opponent_casts_their_card }),
    ..AIMED
};

static BEAR: CardDef = CardDef {
    name: "Caster Bear",
    oracle: true,
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

static FILLER: CardDef = CardDef { name: "Caster Filler", has_cost: false, ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&AIMED, &YOURS, &CASTER_PROBE, &BEAR, &FILLER]))
}

/// A game in my precombat main phase with priority: the probe in my hand, `theirs` in the opponent's graveyard, one {C} in
/// the opponent's pool and none in mine.
fn game_with(theirs: &str) -> (RulesGame, ObjectId, ObjectId) {
    let cards = table();
    let (probe, card, filler) = (cards.find("Caster Probe").unwrap(), cards.find(theirs).unwrap(), cards.find("Caster Filler").unwrap());
    let mut game = RulesGame::new(cards, &Setup { decks: vec![vec![probe, filler, filler, filler, filler, filler, filler], vec![card, filler, filler, filler, filler, filler, filler]], seed: 1, ..Setup::default() });
    let probe_id = game.state.zone(Zone::hand(ME))[0];
    let card_id = game.state.zone(Zone::hand(THEM))[0];
    assert!(game.state.zone_remove(Zone::hand(THEM), card_id));
    game.state.update_obj(card_id, |o| o.zone = Zone::graveyard(THEM));
    game.state.zone_push(Zone::graveyard(THEM), card_id);
    game.state.update_turn(|t| {
        t.number = 1;
        t.active = ME;
        t.priority = ME;
        t.set_step(Step::PrecombatMain);
        t.began = true;
    });
    game.state.add_mana(THEM, Mana::C, 1, ObjectId::NONE, None, ManaLasts::Step);
    game.state.frame_push(Frame::new(FrameKind::Priority));
    (game, probe_id, card_id)
}

fn put(game: &mut RulesGame, name: &str, player: PlayerId) -> ObjectId {
    let card = game.cards.find(name).unwrap();
    let id = game.state.push_obj(Obj::new(card, player, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

/// Casts the probe, then answers every question until the card it has cast is on the stack (or it is clear it was not):
/// each question's player, with what was asked.
fn play_probe(game: &mut RulesGame, probe: ObjectId, card: ObjectId) -> Vec<(PlayerId, &'static str)> {
    game.run();
    let request = game.pending().cloned().unwrap();
    let RequestKind::Priority { options } = &request.kind else { panic!("expected priority") };
    let cast = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(probe)).expect("the probe is castable").clone();
    game.answer(request.id, request.player, Answer::Act(Action::of(&cast))).unwrap();
    let mut asked = Vec::new();
    for _ in 0..32 {
        game.run();
        // The probe on the stack is a new object (CR 400.7).
        let on_stack = game.state.obj(probe).became;
        let probe_gone = !game.state.stack.iter().any(|i| Some(i.object) == on_stack);
        let request = game.pending().cloned().expect("a question");
        let answer = match &request.kind {
            RequestKind::Priority { .. } if probe_gone || game.state.obj(card).became.is_some() => return asked,
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseTarget { candidates, .. } => {
                asked.push((request.player, "targets"));
                Answer::Targets(vec![candidates[0]])
            }
            RequestKind::PayCosts { options, .. } => {
                asked.push((request.player, "pay"));
                let confirm = options
                    .iter()
                    .find(|o| o.kind == ActionKind::AssignMana)
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::ConfirmCasting))
                    .unwrap_or_else(|| panic!("unexpected payment options: {options:?}"));
                Answer::Act(Action::of(confirm))
            }
            other => panic!("unexpected request: {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the probe did not resolve");
}

/// CR 608.2g, 601.2c, 601.2g-h, 405.4: the opponent the effect names casts their card: they choose its target, an opponent
/// of theirs (me, not themselves), and pay its {1} from their own pool; the spell is theirs.
#[test]
fn the_player_an_effect_names_casts_chooses_targets_pays_and_controls_the_spell() {
    let (mut game, probe, card) = game_with("Caster Aimed Instant");
    let asked = play_probe(&mut game, probe, card);
    let spell = game.state.obj(card).became.expect("their card was cast");
    let item = *game.state.stack.iter().find(|i| i.object == spell).expect("the spell is on the stack");
    assert_eq!(item.controller, THEM, "the caster controls the spell");
    assert_eq!(game.state.obj(spell).controller, THEM);
    assert_eq!(game.state.targets(&item), &[Target::Player(ME)], "its \"opponent\" is the caster's: me");
    assert!(!asked.is_empty() && asked.iter().all(|&(p, _)| p == THEM), "every question of the cast is the caster's: {asked:?}");
    assert!(asked.iter().any(|&(_, what)| what == "targets"));
    assert_eq!(game.state.player(THEM).pool.0[Mana::C as usize], 0, "the caster paid the {{1}}");
    let cast = game.log.iter().find_map(|r| match r.event {
        Event::SpellCast { spell: s, player, .. } if s == spell => Some(player),
        _ => None,
    });
    assert_eq!(cast, Some(THEM), "the cast is recorded as theirs");
}

/// CR 608.2g, 601.2c: whether the named player could cast it is judged for them: a spell that targets "a creature you
/// control" can be cast by an opponent who controls a creature though the effect's controller controls none; asked for the
/// effect's controller, it could not.
#[test]
fn whether_the_named_player_could_cast_it_is_judged_for_them() {
    let (mut game, probe, card) = game_with("Caster Your Creature Instant");
    let bear = put(&mut game, "Caster Bear", THEM);
    let life = game.state.player(ME).life;
    play_probe(&mut game, probe, card);
    // Bit 0: `could_cast_during_resolution_by(them)`; bit 1: `could_cast_during_resolution` (for me); bit 2: its ways, for them.
    assert_eq!(game.state.player(ME).life - life, 0b101);
    let spell = game.state.obj(card).became.expect("their card was cast");
    let item = *game.state.stack.iter().find(|i| i.object == spell).unwrap();
    assert_eq!(game.state.targets(&item), &[Target::Object(bear)]);
    assert_eq!(item.controller, THEM);
}
