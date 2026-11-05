use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, targets};
use cantrip_oracle as oracle;

/// "Choose target Wall creature. Whenever that creature is dealt damage by an attacking creature this turn, you gain that much life." (CR 115.1).
fn choose_target_wall_creature_whenever_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Wall)) && q
        .chars(id).is(CardType::Creature)))
    )
}

/// "Whenever that creature is dealt damage by an attacking creature this turn" (CR 603.2). The subject is the Wall dealt the
/// damage, so what several attackers deal it at once is one event for the total (`sums_per_subject`: CR 120.2, 603.2c).
fn watch_1_event(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::Damage { source, target, .. } = event else { return None };
    let source = *source;
    let subject = Target::Object(asker.source);
    let data = [
        0,
        match target {
            Target::Object(o) => o.0,
            _ => u32::MAX,
        },
    ];
    if !q.state().obj(source).zone.is(ZoneKind::Stack)
        && ((q.is_attacking(source) || q.was_attacking(source)) && q.chars(source).is(CardType::Creature))
        && matches!(* target, Target::Object(o) if asker.source == o)
    {
        return Some(Fired { subject, data });
    }
    None
}

/// Set up the delayed ability for the chosen Wall (CR 603.7b).
fn choose_target_wall_creature_whenever(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(wall) = targets::object(g, item, 0) {
        delayed::this_turn(g, item, wall);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[target(choose_target_wall_creature_whenever_target_0)],
        choose_target_wall_creature_whenever,
    ),
    delayed: &[
        triggered(
            when_asked(EventMask::DAMAGE, watch_1_event),
            effects::gain_matched_damage_life,
            "Whenever that creature is dealt damage by an attacking creature this turn, you gain that much life.",
        )
        .damage_amount()
        .sums_per_subject(),
    ],
    ..oracle::GLYPH_OF_LIFE
};
