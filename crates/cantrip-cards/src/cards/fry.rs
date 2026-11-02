use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This spell can't be countered. Fry deals 5 damage to target creature or planeswalker that's white or blue." (CR 115.1).
fn spell_cant_be_countered_fry_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (((q
        .chars(id).is(CardType::Creature) || q.chars(id).is(CardType::Planeswalker)) &&
        (q.chars(id).colors.has(q.words(asker.source).color(Color::White)) || q.chars(id)
        .colors.has(q.words(asker.source).color(Color::Blue)))))
    )
}

/// "This spell can't be countered. Fry deals 5 damage to target creature or planeswalker that's white or blue." (CR 608.2, 120.3).
fn spell_cant_be_countered_fry(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let recipients_0 = t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.object())
        .into_iter()
        .map(Target::Object)
        .collect::<Vec<Target>>();
    let batch: Vec<(ObjectId, Target, u16)> = recipients_0
        .iter()
        .map(|t| (item.object, *t, 5u16))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[target(spell_cant_be_countered_fry_target_0)],
        spell_cant_be_countered_fry,
    ),
    ..oracle::FRY
};
