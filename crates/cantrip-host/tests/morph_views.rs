//! Morph at the table (CR 702.37, 708.4, 708.5), over a test card: the cast face down is offered as such for {3}
//! (`ActionOptionMessage::cast_face_down`), the face-down spell on the stack is a nameless 2/2 to the opponent and the card it
//! is to its controller, and the permanent's turn-face-up action says its morph cost.

use std::collections::HashSet;

use cantrip_engine::*;
use cantrip_host::handles::HandleSpace;
use cantrip_host::prompt::prompt;
use cantrip_host::protocol::*;
use cantrip_host::view::player_view;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static FILLER: CardDef = CardDef { name: "Morph View Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "Flying / Morph {3}{W}": a 4/5 for {4}{W}.
static SKYWARDEN: CardDef = CardDef {
    name: "Morph View Skywarden",
    cost: mana("{4}{W}"),
    types: TypeSet::of(&[CardType::Creature]),
    keywords: Keywords::of(&[Keyword::Flying]),
    power: Some(4),
    toughness: Some(5),
    text: "Flying\nMorph {3}{W}",
    ways: &[face_down_way(WayKind::Morph, Cost::mana(mana("{3}{W}")), "Morph {3}{W}")],
    ..CardDef::BLANK
};

fn game() -> (Game, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &SKYWARDEN])));
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
    let card = g.state.push_obj(Obj::new(table.id_of(&SKYWARDEN), YOU, Zone::hand(YOU)));
    g.state.zone_push(Zone::hand(YOU), card);
    let source = g.state.zone(Zone::library(YOU))[0];
    g.state.add_mana(YOU, Mana::C, 3, source, None, false);
    g.state.add_mana(YOU, Mana::W, 4, source, None, false);
    g.run();
    (g, card)
}

fn options(g: &Game) -> Vec<ActionOptionMessage> {
    let r = g.pending().cloned().expect("a question");
    match prompt(g, &r, &mut HandleSpace::new()).expect("a prompt").0 {
        PromptPayload::PriorityPromptMessage(p) => p.options,
        other => panic!("expected priority, got {other:?}"),
    }
}

fn take(g: &mut Game, pick: impl Fn(&ActionOption) -> bool) {
    loop {
        let r = g.pending().cloned().expect("a question");
        let option = match &r.kind {
            RequestKind::Priority { options } => options.iter().find(|o| pick(o)).expect("offered").clone(),
            RequestKind::PayCosts { options, .. } => {
                options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("the pool pays").clone()
            }
            other => panic!("unexpected {other:?}"),
        };
        g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
        g.run();
        if !matches!(g.pending().map(|r| &r.kind), Some(RequestKind::PayCosts { .. })) {
            return;
        }
    }
}

fn view_of(g: &Game, viewer: PlayerId, id: ObjectId) -> ObjectView {
    let mut handles = HandleSpace::new();
    let names = vec!["You".to_string(), "Them".to_string()];
    let view = player_view(g, viewer, &mut handles, &names, &[false, false], &HashSet::new(), &HashSet::new());
    let handle = handles.existing(id);
    view.objects.into_iter().find(|o| o.handle == handle).expect("the object is in the view")
}

#[test]
fn a_face_down_spell_is_offered_shown_and_turned_up_as_such() {
    let (mut g, card) = game();
    let offered = options(&g);
    let morph = offered.iter().find(|o| o.kind == ActionKindView::CastSpell && o.ability_index == 1).expect("the morph cast");
    assert!(morph.cast_face_down);
    assert_eq!(morph.cost_text, "{3}");
    assert!(morph.description.contains("Morph {3}{W}"), "{:?}", morph.description);
    let normal = offered.iter().find(|o| o.kind == ActionKindView::CastSpell && o.ability_index == 0).expect("the ordinary cast");
    assert!(!normal.cast_face_down);

    take(&mut g, |o| o.kind == ActionKind::CastSpell && o.source == Some(card) && o.index == 1);
    let spell = g.state.stack.last().expect("on the stack").object;
    let theirs = view_of(&g, THEM, spell);
    assert!(theirs.face_down && !theirs.known);
    assert!(theirs.name.is_empty() && theirs.face_down_name.is_empty() && theirs.card_id.is_empty());
    assert_eq!((theirs.power, theirs.toughness), (Some(2), Some(2)));
    assert!(theirs.oracle_text.is_empty());
    let mine = view_of(&g, YOU, spell);
    assert!(mine.face_down && mine.known);
    assert_eq!(mine.face_down_name, "Morph View Skywarden");

    // Resolved, the permanent's special action says what turning it up costs.
    while !g.state.stack.is_empty() {
        let r = g.pending().cloned().expect("a question");
        g.answer(r.id, r.player, Answer::Pass).expect("a legal answer");
        g.run();
    }
    let permanent = g.state.obj(spell).became.expect("it resolved");
    let source = g.state.zone(Zone::library(YOU))[0];
    g.state.add_mana(YOU, Mana::W, 1, source, None, false);
    g.state.add_mana(YOU, Mana::C, 3, source, None, false);
    // The priority offered before the mana was added is offered again.
    let at = g.state.agenda.len() - 1;
    g.state.update_frame(at, |f| f.pending = None);
    g.run();
    let mut handles = HandleSpace::new();
    let handle = handles.handle(permanent);
    let r = g.pending().cloned().expect("a question");
    let PromptPayload::PriorityPromptMessage(p) = prompt(&g, &r, &mut handles).expect("a prompt").0 else { panic!("priority") };
    let up = p.options.iter().find(|o| o.kind == ActionKindView::TurnFaceUp && o.source == handle).expect("turning it up");
    assert_eq!(up.cost_text, "{3}{W}");
    assert!(up.description.contains("Morph {3}{W}"), "{:?}", up.description);
}
