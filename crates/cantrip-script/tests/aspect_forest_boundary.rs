//! Aspect of Wolf uses the full number of controlled Forests before its bounded P/T adjustment.
//! The battlefield is constructed directly to avoid playing tens of thousands of land drops. The counterless
//! Walking Ballista is a stable 0/0 printed creature here: it can have entered with a +1/+1 counter, acquired the Aura,
//! and removed that counter to activate its ability while the Forest bonus kept it alive.
use cantrip_engine::*;
use std::sync::OnceLock;

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all()))
}

fn aspect_at(count: u32) -> (i32, i32) {
    let cards = table();
    let mut game = Game::new(
        cards,
        &Setup {
            decks: vec![vec![], vec![]],
            ..Default::default()
        },
    );
    let state = &mut game.state;
    let controller = PlayerId(0);
    let forest = cards.find("Forest").unwrap();
    for _ in 0..count {
        let id = state.push_obj(Obj::new(forest, controller, Zone::BATTLEFIELD));
        state.zone_push(Zone::BATTLEFIELD, id);
    }
    // Opposing Forests must not contribute to the Aura controller's count.
    for _ in 0..2 {
        let id = state.push_obj(Obj::new(forest, PlayerId(1), Zone::BATTLEFIELD));
        state.zone_push(Zone::BATTLEFIELD, id);
    }
    let host = state.push_obj(Obj::new(
        cards.find("Walking Ballista").unwrap(),
        controller,
        Zone::BATTLEFIELD,
    ));
    state.zone_push(Zone::BATTLEFIELD, host);
    let mut aura = Obj::new(
        cards.find("Aspect of Wolf").unwrap(),
        controller,
        Zone::BATTLEFIELD,
    );
    aura.attached_to = Some(host);
    let aura = state.push_obj(aura);
    state.zone_push(Zone::BATTLEFIELD, aura);
    state.see_statics();
    let chars = game.chars(host);
    (chars.power.unwrap(), chars.toughness.unwrap())
}

#[test]
fn aspect_large_forest_count_uses_the_full_count_before_bounded_pt() {
    for (count, expected) in [
        (65531, (32765, 32766)),
        (65532, (32766, 32766)),
        (65533, (32766, 32767)),
        (65534, (32767, 32767)),
        (65535, (32767, 32768)),
        (65536, (32768, 32768)),
    ] {
        let start = std::time::Instant::now();
        assert_eq!(aspect_at(count), expected, "{count} controlled Forests");
        eprintln!("{count} controlled Forest objects: {:?}", start.elapsed());
    }
}
