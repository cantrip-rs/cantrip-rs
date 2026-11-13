use cantrip_api::*;
use cantrip_mechanics::{statics, triggers};
use cantrip_oracle as oracle;

/// "Put X glyph counters on target creature that target Wall blocked this turn, where X is the power of that blocked creature. The creature gains "This creature doesn't untap during your untap step if it has a glyph counter on it" and "At the beginning of your upkeep, remove a glyph counter from this creature."" (CR 115.1).
fn put_x_glyph_counters_target_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (q
        .chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Wall)))
    )
}

/// "Put X glyph counters on target creature that target Wall blocked this turn, where X is the power of that blocked creature. The creature gains "This creature doesn't untap during your untap step if it has a glyph counter on it" and "At the beginning of your upkeep, remove a glyph counter from this creature."" (CR 115.1).
fn put_x_glyph_counters_target_target_1(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Creature) && ({ let history_candidate = id; q.history()
        .this_turn().iter().filter_map(| r : & Record | -> Option < Fired > { let event =
        & r.event; if let Event::Blocks { blocker, attacker, .. } = event { let id = *
        attacker; let other = * blocker; let subject = Target::Object(id); if
        history_candidate == id && q.chosen_targets().get(0).copied().and_then(| t :
        Target | t.object()) == Some(other) { return Fired::about(subject); } } None })
        .count() as i64 }) >= 1))
    )
}

/// "This creature doesn't untap during your untap step if it has a glyph counter on it" (CR 604.1).
fn granted_2_0_0_keyword(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let asker = &Asker::new(source, q.controller(source));
    if Some(asker.source)
        .is_some_and(|candidate: ObjectId| {
            (i64::from(q.state().obj(candidate).counters(CounterKind::Glyph))) > 0
        })
    {
        chars.keywords = chars.keywords.with(Keyword::DoesNotUntap);
    }
}

/// "At the beginning of your upkeep, remove a glyph counter from this creature." (CR 608.2).
fn granted_3(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_4_0 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    instruction(
        g,
        |g| {
            for object in objects_4_0 {
                let _ = g.remove_counters(object, CounterKind::Glyph, 1);
            }
        },
    );
    Ok(())
}

/// "Put X glyph counters on target creature that target Wall blocked this turn, where X is the power of that blocked creature. The creature gains "This creature doesn't untap during your untap step if it has a glyph counter on it" and "At the beginning of your upkeep, remove a glyph counter from this creature."" (CR 608.2, 611.2a, 122.1).
fn put_x_glyph_counters_target(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_1_raw = announced(g, item, 1, false);
    let t_1 = announced(g, item, 1, true);
    let mut counters_0: Vec<(ObjectId, CounterKind, i32)> = Vec::new();
    let amount = {
        let q: &dyn Query = g;
        t_1_raw
            .get(0)
            .copied()
            .and_then(|t: Target| t.object())
            .map_or(0, |o: ObjectId| i64::from(q.chars(o).power.unwrap_or(0)))
            .max(0) as i32
    };
    for t in t_1_raw
        .get(0)
        .copied()
        .filter(|t| t_1.contains(t))
        .and_then(|t: Target| t.object())
        .into_iter()
        .map(Target::Object)
        .collect::<Vec<Target>>()
    {
        if let Target::Object(object) = t {
            counters_0.push((object, CounterKind::Glyph, amount));
        }
    }
    g.add_counters_together(&counters_0);
    g.end_sentence();
    let objects_1 = {
        let q: &dyn Query = g;
        t_1_raw
            .get(0)
            .copied()
            .filter(|t| t_1.contains(t))
            .and_then(|t: Target| t.object())
            .into_iter()
            .filter(|&o| is_live(q, o))
            .collect::<Vec<ObjectId>>()
    };
    let kinds_1 = {
        let mut all = Vec::new();
        for &object in &objects_1 {
            let kind = {
                let q: &dyn Query = g;
                EffectKind::AddAbility(
                    AbilityRef::delayed(
                        {
                            let o = q.state().obj(item.object);
                            o.copied_card().unwrap_or(o.card)
                        },
                        0,
                    ),
                )
            };
            all.push((object, kind));
            let kind = {
                let q: &dyn Query = g;
                EffectKind::AddAbility(
                    AbilityRef::delayed(
                        {
                            let o = q.state().obj(item.object);
                            o.copied_card().unwrap_or(o.card)
                        },
                        1,
                    ),
                )
            };
            all.push((object, kind));
        }
        all
    };
    instruction(
        g,
        |g| {
            for &(object, kind) in &kinds_1 {
                g.add_effect(item.object, object, kind, Duration::Indefinite);
            }
        },
    );
    Ok(())
}

/// "Put X glyph counters on target creature that target Wall blocked this turn, where X is the power of that blocked creature. The creature gains "This creature doesn't untap during your untap step if it has a glyph counter on it" and "At the beginning of your upkeep, remove a glyph counter from this creature."" (CR 608.2).
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
            target(put_x_glyph_counters_target_target_0),
            target(put_x_glyph_counters_target_target_1),
        ],
        put_x_glyph_counters_target,
    ),
    delayed: &[
        AbilityDef::Static(
            static_ability(
                Layer::Ability,
                statics::itself,
                granted_2_0_0_keyword,
                "This creature doesn't untap during your untap step if it has a glyph counter on it",
            ),
        ),
        triggered(
            triggers::YOUR_UPKEEP,
            granted_3,
            "At the beginning of your upkeep, remove a glyph counter from this creature.",
        ),
    ],
    ..oracle::GLYPH_OF_DELUSION
};
