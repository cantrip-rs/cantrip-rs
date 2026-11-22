//! A creature type chosen in secret as each player is shown it (CR 607.2d): to the player who chose it, and to no one else.

use std::collections::HashSet;

use cantrip_engine::*;
use cantrip_host::handles::HandleSpace;
use cantrip_host::protocol::*;
use cantrip_host::view::player_view;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static FILLER: CardDef = CardDef { name: "Secret View Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "{0}: Secretly choose Human or Goblin."
fn choose_in_secret(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let ask = RequestKind::ChooseSubtype { source: item.object, choices: vec![Subtype::Human, Subtype::Goblin], purpose: WordUse::Chosen };
    let Answer::Subtype(chosen) = g.ask(item.controller, ask)? else { unreachable!("a subtype") };
    let by = g.linked(item);
    g.remember_secret_subtype(by, chosen, item.controller);
    Ok(())
}

static ENCHANTMENT: CardDef = CardDef {
    name: "Secret View Enchantment",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[ability(Cost::NONE, choose_in_secret, "{0}: Secretly choose Human or Goblin.")],
    ..CardDef::BLANK
};

fn game() -> (Game, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &ENCHANTMENT])));
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
    g.state.update_player(YOU, |p| p.turn_began = 1);
    let id = g.state.push_obj(Obj::new(table.id_of(&ENCHANTMENT), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
    (g, id)
}

/// What `viewer` is shown of `object`'s choices, as (key, value).
fn choices_seen(g: &Game, viewer: PlayerId, object: &str) -> Vec<(String, String)> {
    let names = vec!["You".to_string(), "Them".to_string()];
    let view = player_view(g, viewer, &mut HandleSpace::new(), &names, &[false, false], &HashSet::new(), &HashSet::new(), &HashSet::new());
    let shown = view.objects.iter().find(|o| o.name == object && o.zone == Some(ZoneKindView::Battlefield)).expect("it is on the battlefield");
    shown.choices.iter().map(|c| (c.key.clone(), c.value.clone())).collect()
}

fn choose(g: &mut Game, source: ObjectId, subtype: Subtype) {
    g.run();
    let r = g.pending().cloned().expect("priority");
    let RequestKind::Priority { options } = &r.kind else { panic!("priority") };
    let option = options.iter().find(|o| o.source == Some(source)).expect("the ability is offered");
    g.answer(r.id, r.player, Answer::Act(Action::of(option))).expect("a legal answer");
    loop {
        g.run();
        let r = g.pending().cloned().expect("a question");
        match &r.kind {
            RequestKind::Priority { .. } if g.state.stack.is_empty() => break,
            RequestKind::Priority { .. } => g.answer(r.id, r.player, Answer::Pass).expect("a legal answer"),
            RequestKind::ChooseSubtype { .. } => g.answer(r.id, r.player, Answer::Subtype(subtype)).expect("a legal answer"),
            RequestKind::PayCosts { options, .. } => {
                let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("it costs nothing").clone();
                g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}

/// The player who chose is shown the type; the other player is shown nothing of it - in no key, no value.
#[test]
fn only_the_player_who_chose_is_shown_the_type() {
    let (mut g, enchantment) = game();
    choose(&mut g, enchantment, Subtype::Goblin);
    assert_eq!(choices_seen(&g, YOU, "Secret View Enchantment"), [("secret creature type".to_string(), "Goblin".to_string())]);
    assert_eq!(choices_seen(&g, THEM, "Secret View Enchantment"), []);
}
