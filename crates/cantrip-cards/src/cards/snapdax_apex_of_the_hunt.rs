use cantrip_api::*;
use cantrip_mechanics::mutate;
use cantrip_oracle as oracle;

/// "Whenever this creature mutates, it deals 4 damage to target creature or planeswalker an opponent controls and you gain 4 life." (CR 115.1).
fn deals_4_damage_target_creature_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (((q
        .chars(id).is(CardType::Creature) || q.chars(id).is(CardType::Planeswalker)) && q
        .state().opponents(asker.controller).collect:: < Vec < PlayerId > > ().contains(&
        q.controller(id))))
    )
}

/// "Whenever this creature mutates" (CR 603.2).
fn when_creature_mutates(_: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::Mutated { permanent, .. } = event else { return None };
    let id = *permanent;
    let subject = Target::Object(id);
    if asker.source == id {
        return Fired::about(subject);
    }
    None
}

/// "Whenever this creature mutates, it deals 4 damage to target creature or planeswalker an opponent controls and you gain 4 life." (CR 608.2, 120.3, 119.3).
fn deals_4_damage_target_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let source_0 = item.subject.object();
    let recipients_0 = t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.object())
        .into_iter()
        .map(Target::Object)
        .collect::<Vec<Target>>();
    let Some(source) = source_0 else { return Ok(()) };
    let batch: Vec<(ObjectId, Target, u16)> = recipients_0
        .iter()
        .map(|t| (source, *t, 4u16))
        .collect();
    g.damage_together(&batch);
    g.end_sentence();
    g.gain_life(item.controller, 4);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                when_asked(EventMask::MUTATED, when_creature_mutates),
                deals_4_damage_target_creature,
                "Whenever this creature mutates, it deals 4 damage to target creature or planeswalker an opponent controls and you gain 4 life.",
            )
            .targets(&[target(deals_4_damage_target_creature_target_0)]),
    ],
    ways: &[
        mutate::way(
            ManaCost {
                generic: 2,
                x: 0,
                symbols: [2, 0, 0, 0, 0, 0],
                hybrid: [Some((Mana::B, Mana::R)), None, None, None, None, None],
                twobrid: [0, 0, 0, 0, 0],
                phyrexian: [0, 0, 0, 0, 0],
            },
            "Mutate {2}{B/R}{W}{W}",
        ),
    ],
    ..oracle::SNAPDAX_APEX_OF_THE_HUNT
};
