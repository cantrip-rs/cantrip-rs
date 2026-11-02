use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target creature gets +2/+0 and gains haste until end of turn." (CR 115.1).
fn target_creature_gets_2_0_target_0(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (q
        .chars(id).is(CardType::Creature))
    )
}

/// "Target creature gets +2/+0 and gains haste until end of turn." (CR 608.2, 611.2a).
fn target_creature_gets_2_0(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_0 = targets::object(g, item, 0);
    let kind_0_1_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::Haste]));
    instruction(
        g,
        |g| {
            if let Some(object) = objects_0 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::Pump(2, 0),
                    Duration::EndOfTurn,
                );
                g.add_effect(item.object, object, kind_0_1_0, Duration::EndOfTurn);
            }
        },
    );
    Ok(())
}

pub static FAME: CardDef = CardDef {
    spell: spell(&[target(target_creature_gets_2_0_target_0)], target_creature_gets_2_0),
    aftermath: true,
    ..oracle::FAME
};

/// "Return target creature card with mana value 2 or less from your graveyard to the battlefield." (CR 115.1).
fn return_target_creature_card_mana_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if (q.state().obj(id).zone.kind() == ZoneKind::Graveyard)
        && ! q.state().obj(id).token && (((q.chars(id).is(CardType::Creature) &&
        (i64::from(q.mana_value(id))) <= 2) && (q.state().obj(id).zone.kind() ==
        ZoneKind::Graveyard && asker.controller == q.state().obj(id).owner)))
    )
}

/// "Return target creature card with mana value 2 or less from your graveyard to the battlefield." (CR 608.2).
fn return_target_creature_card_mana(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_0 = t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.object())
        .into_iter()
        .filter(|&o: &ObjectId| !g.state().obj(o).token)
        .collect::<Vec<ObjectId>>();
    let rows_0: Vec<(ObjectId, PlayerId, bool)> = objects_0
        .iter()
        .map(|&o| (o, item.controller, false))
        .collect();
    g.put_all_onto_battlefield_as(&rows_0)?;
    Ok(())
}

pub static CLAIM: CardDef = CardDef {
    spell: spell(
        &[target(return_target_creature_card_mana_target_0)],
        return_target_creature_card_mana,
    ),
    ..oracle::CLAIM
};

/// "Return target creature card with mana value 2 or less from your graveyard to the battlefield." (CR 604.1).
pub static CARD: CardDef = CardDef {
    split: Some((&CLAIM, &FAME)),
    ..oracle::CLAIM_FAME
};

pub static EXTRA: &[&CardDef] = &[&CLAIM, &FAME];
