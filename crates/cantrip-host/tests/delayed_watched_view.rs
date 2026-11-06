use std::collections::HashSet;

use cantrip_api::*;
use cantrip_engine::{Game, Setup};
use cantrip_host::{handles::HandleSpace, view::player_view};
use cantrip_state::{Delayed, Span};

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> { Ok(()) }
static SOURCE: CardDef = CardDef {
    name: "Public delayed spell", types: TypeSet::of(&[CardType::Instant]),
    delayed: &[delayed_trigger(nothing, "When that Goblin dies this turn, draw a card.")], ..CardDef::BLANK
};
static BODY: CardDef = CardDef {
    name: "Matching creature", types: TypeSet::of(&[CardType::Creature]),
    power: Some(1), toughness: Some(1), ..CardDef::BLANK
};

#[test]
fn delayed_watcher_marks_only_the_captured_battlefield_object_for_both_seats() {
    let cards = Box::leak(Box::new(CardTable::new([&SOURCE, &BODY])));
    let mut game = Game::new(cards, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    let you = PlayerId(0);
    let mut put = |def, zone| {
        let id = game.state.push_obj(Obj::new(cards.id_of(def), you, zone));
        game.state.zone_push(zone, id);
        id
    };
    let source = put(&SOURCE, Zone::graveyard(you));
    let watched = put(&BODY, Zone::BATTLEFIELD);
    let other = put(&BODY, Zone::BATTLEFIELD);
    let hidden = put(&BODY, Zone::hand(you));
    game.state.delayed_push(Delayed {
        when: DelayedWhen::ThisTurnWatching(watched), source, controller: you,
        ability: AbilityRef::delayed(cards.id_of(&SOURCE), 0), subject: Target::None,
        members: Span::default(), since: 0, acquired: Acquired::OWN,
        words: Some(Words::NONE.changed(Word::Subtype(Subtype::Goblin), Word::Subtype(Subtype::Elf))),
    });
    let empty = HashSet::new();
    let names = ["You".to_string(), "Opponent".to_string()];
    for viewer in [PlayerId(0), PlayerId(1)] {
        let mut handles = HandleSpace::new();
        let view = player_view(&game, viewer, &mut handles, &names, &[false, false], &empty, &empty, &empty);
        let affected = handles.handle(watched);
        let unaffected = handles.handle(other);
        let ours = view.objects.iter().find(|o| o.handle == affected).unwrap();
        let theirs = view.objects.iter().find(|o| o.handle == unaffected).unwrap();
        let mark = &ours.granted_abilities.iter().find(|a| a.text.contains("Pending delayed ability")).unwrap().text;
        assert!(mark.contains("Public delayed spell") && mark.contains("this turn") && mark.contains("Elf") && !mark.contains("Goblin"), "{mark}");
        assert!(theirs.granted_abilities.iter().all(|a| !a.text.contains("Pending delayed ability")));
        if viewer == you {
            let hand = handles.handle(hidden);
            assert!(view.objects.iter().find(|o| o.handle == hand).unwrap().granted_abilities.iter().all(|a| !a.text.contains("Pending delayed ability")));
        } else {
            assert!(!view.objects.iter().any(|o| o.zone == Some(cantrip_host::protocol::ZoneKindView::Hand) && o.name == "Matching creature"));
        }
    }
}
