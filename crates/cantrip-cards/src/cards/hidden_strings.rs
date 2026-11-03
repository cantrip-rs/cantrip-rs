use cantrip_api::*;
use cantrip_mechanics::{choose, cipher};
use cantrip_oracle as oracle;

/// "You may tap or untap target permanent, then you may tap or untap another target permanent." (CR 115.1).
fn tap_untap_target_permanent_tap_target_0(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (true)
    )
}

/// "You may tap or untap target permanent, then you may tap or untap another target permanent." (CR 115.1).
fn tap_untap_target_permanent_tap_target_1(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((! (q
        .chosen_targets().get(0).copied().and_then(| t : Target | t.object()) ==
        Some(id))))
    )
}

/// "You may tap or untap target permanent, then you may tap or untap another target permanent." (CR 608.2, 701.26a, 502.3).
fn tap_untap_target_permanent_tap(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0_raw = announced(g, item, 0, false);
    let t_0 = announced(g, item, 0, true);
    let t_1_raw = announced(g, item, 1, false);
    let t_1 = announced(g, item, 1, true);
    if let Some(object) = ({
        let q: &dyn Query = g;
        t_0_raw
            .get(0)
            .copied()
            .filter(|t| t_0.contains(t))
            .and_then(|t: Target| t.object())
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    })
        .first()
        .copied()
    {
        let card = g.card_id(&CARD);
        match choose::one_of(
            g,
            item.controller,
            item.object,
            vec![Label::Card(card, 0), Label::Card(card, 1), Label::Card(card, 2)],
        )? {
            0 => g.tap(object),
            1 => g.untap(object),
            _ => {}
        };
    }
    g.end_sentence();
    g.end_sentence();
    if let Some(object) = ({
        let q: &dyn Query = g;
        t_1_raw
            .get(0)
            .copied()
            .filter(|t| t_1.contains(t))
            .and_then(|t: Target| t.object())
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    })
        .first()
        .copied()
    {
        let card = g.card_id(&CARD);
        match choose::one_of(
            g,
            item.controller,
            item.object,
            vec![Label::Card(card, 0), Label::Card(card, 1), Label::Card(card, 2)],
        )? {
            0 => g.tap(object),
            1 => g.untap(object),
            _ => {}
        };
    }
    g.end_sentence();
    g.end_sentence();
    cipher::encode(g, item)?;
    Ok(())
}

/// "You may tap or untap target permanent, then you may tap or untap another target permanent." (CR 608.2).
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
        &[
            target(tap_untap_target_permanent_tap_target_0),
            target(tap_untap_target_permanent_tap_target_1),
        ],
        tap_untap_target_permanent_tap,
    ),
    statics: &[cipher::STATIC],
    delayed: &[cipher::TRIGGER],
    labels: choose::TAP_OR_UNTAP,
    ..oracle::HIDDEN_STRINGS
};
