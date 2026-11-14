//! Fixed printed costs feeding battlefield mana value must fit even when two faces are combined
//! (meld or both Room doors); X is zero for a battlefield incarnation.
#[test]
fn registered_printed_costs_leave_room_for_two_faces() {
    for card in &cantrip_engine::cards().cards {
        for face in std::iter::once(*card).chain(card.split.into_iter().flat_map(|(left, right)| [left, right])) {
            let value = face.cost.mana_value();
            assert!(value <= i32::MAX as u32 / 2, "{} has mana value {value}", face.name);
        }
    }
}
