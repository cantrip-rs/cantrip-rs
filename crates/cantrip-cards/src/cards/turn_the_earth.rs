use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Choose up to three target cards in graveyards. The owners of those cards shuffle them into their libraries. You gain 2 life." (CR 115.1).
fn choose_up_three_target_cards_target_0(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if (q.state().obj(id).zone.kind() == ZoneKind::Graveyard)
        && ! q.state().obj(id).token && (true)
    )
}

/// "Choose up to three target cards in graveyards. The owners of those cards shuffle them into their libraries. You gain 2 life." (CR 608.2, 119.3).
fn choose_up_three_target_cards(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = announced(g, item, 0, true);
    let objects_0 = t_0.iter().filter_map(|t| t.object()).collect::<Vec<ObjectId>>();
    g.put_into_library(&objects_0, LibraryPlace::ShuffledIn);
    g.end_sentence();
    g.gain_life(item.controller, 2);
    Ok(())
}

/// "Choose up to three target cards in graveyards. The owners of those cards shuffle them into their libraries. You gain 2 life." (CR 608.2).
fn choose_up_three_target_cards_require(q: &dyn Query, asker: &Asker) -> bool {
    q.chars(asker.source).is(CardType::Instant)
}

/// "Choose up to three target cards in graveyards. The owners of those cards shuffle them into their libraries. You gain 2 life." (CR 608.2).
fn announced(g: &dyn Game, item: &StackItem, spec: u8, legal: bool) -> Vec<Target> {
    (0..g.state().targets(item).len())
        .filter(|&i| {
            g.state().spec(item, i) == spec && (!legal || g.target_legal(item, i))
        })
        .map(|i| g.state().target(item, i))
        .collect()
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[target(choose_up_three_target_cards_target_0).between(0, 3)],
        choose_up_three_target_cards,
    ),
    ways: &[
        way(cost("{1}{G}"), ZoneKind::Graveyard, "Flashback")
            .only_from()
            .exile_after()
            .condition(choose_up_three_target_cards_require),
    ],
    ..oracle::TURN_THE_EARTH
};
