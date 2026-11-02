use cantrip_api::*;
use cantrip_oracle as oracle;

/// "When this creature enters, return up to one other target nonland permanent you control to its owner's hand. If it was a token, draw a card." (CR 115.1).
fn return_up_one_other_target_target_0(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((! (q
        .event_subject().object() == Some(id)) && (! (q.chars(id).is(CardType::Land)) &&
        asker.controller == q.controller(id))))
    )
}

/// "When this creature enters" (CR 603.2).
fn when_creature_enters(_: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { new, to, .. } = event else { return None };
    let id = *new;
    let subject = Target::Object(id);
    if to.is(ZoneKind::Battlefield) && asker.source == id {
        return Fired::about(subject);
    }
    None
}

/// "When this creature enters, return up to one other target nonland permanent you control to its owner's hand. If it was a token, draw a card." (CR 608.2, 121.1, 400.7).
fn return_up_one_other_target(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0_raw = announced(g, item, 0, false);
    let t_0 = announced(g, item, 0, true);
    let objects_0 = t_0_raw
        .get(0)
        .copied()
        .filter(|t| t_0.contains(t))
        .and_then(|t: Target| t.object())
        .into_iter()
        .collect::<Vec<_>>();
    g.return_to_hand(&objects_0);
    g.end_sentence();
    if {
        let q: &dyn Query = g;
        (objects_0
            .iter()
            .copied()
            .filter(|&o| q.state().obj(o).became.is_some())
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .filter(|&id| q.state().obj(id).token)
            .collect::<Vec<ObjectId>>()
            .len() as i64) > 0
    } {
        g.draw(item.controller, 1);
        g.end_sentence();
    }
    Ok(())
}

/// "When this creature enters, return up to one other target nonland permanent you control to its owner's hand. If it was a token, draw a card." (CR 608.2).
fn announced(g: &dyn Game, item: &StackItem, spec: u8, legal: bool) -> Vec<Target> {
    (0..g.state().targets(item).len())
        .filter(|&i| {
            g.state().spec(item, i) == spec && (!legal || g.target_legal(item, i))
        })
        .map(|i| g.state().target(item, i))
        .collect()
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                when_asked(EventMask::ZONE_CHANGE, when_creature_enters),
                return_up_one_other_target,
                "When this creature enters, return up to one other target nonland permanent you control to its owner's hand. If it was a token, draw a card.",
            )
            .targets(&[target(return_up_one_other_target_target_0).between(0, 1)]),
    ],
    ..oracle::SUNPEARL_KIRIN
};
