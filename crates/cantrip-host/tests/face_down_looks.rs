//! Looking at others' face-down permanents (CR 708.5, `PlayRule::LookAtFaceDown`), over test cards: a seat a rule lets look
//! sees a face-down permanent's face in its view (the shared `visibility::knows_face`, which the AI's observation and worlds
//! read as well), while the rule applies and only on the battlefield (a
//! face-down spell stays its controller's alone), and the player controlling it sees it too (CR 723.4); the public never does.
//! A rule a resolved effect gave (`EffectKind::PlayerPlayRule`) shows it the same way, a permanent turned face down after the
//! effect began too, and nothing once the effect is gone.

use std::collections::HashSet;

use cantrip_engine::visibility::{PUBLIC, knows_face};
use cantrip_engine::*;
use cantrip_host::handles::HandleSpace;
use cantrip_host::view::player_view;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static FILLER: CardDef = CardDef { name: "Face-Down Look Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "Morph {3}": a 3/3 for {3}{G}.
static HIDDEN: CardDef = CardDef {
    name: "Face-Down Look Hidden",
    cost: mana("{3}{G}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    text: "Morph {3}",
    ways: &[face_down_way(WayKind::Morph, Cost::mana(mana("{3}")), "Morph {3}")],
    ..CardDef::BLANK
};

fn opponents_creature(q: &dyn Query, a: &Asker, id: ObjectId) -> bool {
    q.state().obj(id).controller != a.controller && q.chars(id).is(CardType::Creature)
}

/// "You may look at face-down creatures your opponents control any time."
static LOOKER: CardDef = CardDef {
    name: "Face-Down Look Looker",
    types: TypeSet::of(&[CardType::Artifact]),
    text: "You may look at face-down creatures your opponents control any time.",
    play_rules: &[PlayRule::LookAtFaceDown(opponents_creature)],
    ..CardDef::BLANK
};

/// What gives "you may look at face-down creatures you don't control any time" until end of turn.
static GIVER: CardDef = CardDef {
    name: "Face-Down Look Giver",
    types: TypeSet::of(&[CardType::Artifact]),
    delayed: &[granted_play_rule(PlayRule::LookAtFaceDown(opponents_creature), "You may look at face-down creatures you don't control any time.")],
    ..CardDef::BLANK
};

fn game() -> (Game, &'static CardTable) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &HIDDEN, &LOOKER, &GIVER])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.run();
    (g, table)
}

fn put(g: &mut Game, def: CardId, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(def, owner, zone));
    g.state.zone_push(zone, id);
    id
}

/// A face-down 2/2 of `HIDDEN` that `owner` controls on the battlefield, as a morph spell that resolved is.
fn face_down(g: &mut Game, table: &CardTable, owner: PlayerId) -> ObjectId {
    let id = put(g, table.id_of(&HIDDEN), owner, Zone::BATTLEFIELD);
    let values = table.face_down_values(FaceDown::Manifested);
    g.state.update_obj(id, |o| {
        o.face_down = true;
        o.copy = Some(Copiable::of(values));
    });
    id
}

/// What `viewer`'s view says of `id`: whether it knows the face, and the name it gives it.
fn seen(g: &Game, viewer: PlayerId, id: ObjectId) -> (bool, String) {
    let mut handles = HandleSpace::new();
    let names = vec!["You".to_string(), "Them".to_string()];
    let view = player_view(g, viewer, &mut handles, &names, &[false, false], &HashSet::new(), &HashSet::new(), &HashSet::new());
    let handle = handles.existing(id);
    let object = view.objects.into_iter().find(|o| o.handle == handle).expect("the object is in the view");
    assert!(object.face_down);
    assert_eq!(object.known, knows_face(g, viewer, id), "the view and the shared visibility agree");
    (object.known, object.face_down_name)
}

#[test]
fn a_rule_lets_its_controller_look_at_face_down_creatures_opponents_control() {
    let (mut g, table) = game();
    let theirs = face_down(&mut g, table, THEM);
    let mine = face_down(&mut g, table, YOU);
    assert_eq!(seen(&g, YOU, theirs), (false, String::new()), "CR 708.5: hidden without a rule");

    let looker = put(&mut g, table.id_of(&LOOKER), YOU, Zone::BATTLEFIELD);
    let name = HIDDEN.name.to_string();
    assert_eq!(seen(&g, YOU, theirs), (true, name.clone()));
    assert!(!knows_face(&g, THEM, mine), "the rule is its controller's alone");
    assert!(!knows_face(&g, PUBLIC, theirs), "the public sees no player's look");
    assert_eq!(seen(&g, THEM, mine), (false, String::new()));

    // Gone, it shows nothing more (the ruling: "you can no longer look at them").
    g.state.zone_remove(Zone::BATTLEFIELD, looker);
    g.state.update_obj(looker, |o| o.zone = Zone::GONE);
    assert_eq!(seen(&g, YOU, theirs), (false, String::new()));
}

#[test]
fn a_face_down_spell_stays_hidden_from_a_player_who_may_look_at_permanents() {
    let (mut g, table) = game();
    put(&mut g, table.id_of(&LOOKER), YOU, Zone::BATTLEFIELD);
    let spell = put(&mut g, table.id_of(&HIDDEN), THEM, Zone::STACK);
    let values = table.face_down_values(FaceDown::Manifested);
    g.state.update_obj(spell, |o| {
        o.face_down = true;
        o.copy = Some(Copiable::of(values));
    });
    assert!(!g.may_look_at_face_down(YOU, spell));
    assert!(!knows_face(&g, YOU, spell));
}

#[test]
fn a_rule_an_effect_gave_shows_what_turns_face_down_later_until_the_effect_ends() {
    let (mut g, table) = game();
    let giver = put(&mut g, table.id_of(&GIVER), YOU, Zone::BATTLEFIELD);
    let timestamp = g.state.timestamp();
    g.state.effect_push(ContinuousEffect {
        source: giver,
        controller: YOU,
        timestamp,
        affected: giver,
        kind: EffectKind::PlayerPlayRule(AbilityRef::delayed(table.id_of(&GIVER), 0)),
        duration: Duration::EndOfTurn,
        words: Words::NONE,
    });
    // Turned face down after the effect began, and its source gone: the rule is the player's, about what is face down now.
    g.state.zone_remove(Zone::BATTLEFIELD, giver);
    g.state.update_obj(giver, |o| o.zone = Zone::GONE);
    let later = face_down(&mut g, table, THEM);
    let name = HIDDEN.name.to_string();
    assert_eq!(seen(&g, YOU, later), (true, name));

    g.state.effects_retain(|_| false);
    assert_eq!(seen(&g, YOU, later), (false, String::new()), "the turn over, it is hidden again");
}
