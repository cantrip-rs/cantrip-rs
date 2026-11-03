use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Create a token that's a copy of target creature you control." (CR 115.1).
fn create_token_thats_copy_target_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Creature) && asker.controller == q.controller(id)))
    )
}

/// "Create a token that's a copy of target creature you control." (CR 608.2, 111.1).
fn create_token_thats_copy_target(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(original) = targets::object(g, item, 0) else { return Ok(()) };
    g.create_token_copies(original, item.controller, 1)?;
    Ok(())
}

pub static REPLICATE: CardDef = CardDef {
    spell: spell(
        &[target(create_token_thats_copy_target_target_0)],
        create_token_thats_copy_target,
    ),
    ..oracle::REPLICATE
};

/// "Counter target activated or triggered ability. (Mana abilities can't be targeted.)" (CR 115.1).
fn counter_target_activated_triggered_ability_target_0(
    q: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    q.state()
        .stack
        .iter()
        .any(|e| {
            matches!(
                t, Target::Ability(serial) if e.kind != StackKind::Spell && e.serial ==
                serial
            ) && ((e.kind == StackKind::Activated || e.kind == StackKind::Triggered))
        })
}

/// "Counter target activated or triggered ability. (Mana abilities can't be targeted.)" (CR 608.2, 701.5a).
fn counter_target_activated_triggered_ability(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let items_0: Vec<(ObjectId, usize)> = t_0
        .get(0)
        .copied()
        .into_iter()
        .filter_map(|t| {
            let stack = &g.state().stack;
            stack
                .iter()
                .position(|e| {
                    matches!(
                        t, Target::Ability(serial) if e.kind != StackKind::Spell && e
                        .serial == serial
                    )
                })
                .map(|at| (stack[at].object, at))
        })
        .collect();
    g.counter_items(&items_0);
    Ok(())
}

pub static REPUDIATE: CardDef = CardDef {
    spell: spell(
        &[target(counter_target_activated_triggered_ability_target_0)],
        counter_target_activated_triggered_ability,
    ),
    ..oracle::REPUDIATE
};

/// "Counter target activated or triggered ability. (Mana abilities can't be targeted.)" (CR 604.1).
pub static CARD: CardDef = CardDef {
    split: Some((&REPUDIATE, &REPLICATE)),
    ..oracle::REPUDIATE_REPLICATE
};

pub static EXTRA: &[&CardDef] = &[&REPUDIATE, &REPLICATE];
