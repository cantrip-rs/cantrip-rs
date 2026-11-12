use cantrip_engine::{visibility::PUBLIC, *};
use cantrip_host::{handles::HandleSpace, protocol::ZoneKindView, view::player_view};
use std::collections::HashSet;

fn own(_: &dyn Query, a: &Asker, p: PlayerId) -> bool {
    a.controller == p
}
fn all(_: &dyn Query, _: &Asker, _: PlayerId) -> bool {
    true
}
static OWN: CardDef = CardDef {
    name: "Host own public hand",
    play_rules: &[PlayRule::HandRevealed(own)],
    ..CardDef::BLANK
};
static ALL: CardDef = CardDef {
    name: "Host all public hands",
    play_rules: &[PlayRule::HandRevealed(all)],
    ..CardDef::BLANK
};
static CARD: CardDef = CardDef {
    name: "Host hand card",
    ..CardDef::BLANK
};

fn check(def: &'static CardDef, public_owners: &[i32]) {
    let table = Box::leak(Box::new(CardTable::new([&OWN, &ALL, &CARD])));
    let mut g = Game::new(
        table,
        &Setup {
            decks: vec![vec![], vec![]],
            ..Default::default()
        },
    );
    for p in [PlayerId(0), PlayerId(1)] {
        let zone = Zone::hand(p);
        let id = g.state.push_obj(Obj::new(table.id_of(&CARD), p, zone));
        g.state.zone_push(zone, id);
    }
    let source = g
        .state
        .push_obj(Obj::new(table.id_of(def), PlayerId(0), Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, source);
    for open in [true, false] {
        if !open {
            g.state.zone_remove(Zone::BATTLEFIELD, source);
            g.state.update_obj(source, |o| o.zone = Zone::GONE);
        }
        for viewer in [PlayerId(0), PlayerId(1), PUBLIC] {
            let mut handles = HandleSpace::new();
            let empty = HashSet::new();
            let view = player_view(
                &g,
                viewer,
                &mut handles,
                &["A".into(), "B".into()],
                &[false, false],
                &empty,
                &empty,
                &empty,
            );
            for zone in view
                .zones
                .iter()
                .filter(|z| z.kind == Some(ZoneKindView::Hand))
            {
                let visible = (viewer != PUBLIC && zone.owner == i32::from(viewer.0) + 1)
                    || (open && public_owners.contains(&zone.owner));
                assert_eq!(
                    zone.contents_visible, visible,
                    "viewer {viewer:?}, {zone:?}"
                );
                assert_eq!(zone.contents.len(), usize::from(visible));
                assert_eq!(zone.count, 1);
                if visible {
                    assert!(
                        view.objects.iter().any(|o| o.handle == zone.contents[0]
                            && o.known
                            && o.name == CARD.name)
                    );
                }
            }
        }
    }
}

#[test]
fn continuously_public_hands_have_visible_contents_and_named_handles() {
    check(&ALL, &[1, 2]);
}
#[test]
fn opening_only_the_controllers_hand_does_not_expose_the_opponents() {
    check(&OWN, &[1]);
}
