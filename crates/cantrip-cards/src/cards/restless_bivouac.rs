use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "This land enters tapped. {T}: Add {R} or {W}. {1}{R}{W}: This land becomes a 2/2 red and white Ox creature until end of turn. It's still a land. Whenever this land attacks, put a +1/+1 counter on target creature you control." (CR 608.2, 701.26a).
fn add_r_w_1_r_enters(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.tapped = true;
    Ok(())
}

/// "{1}{R}{W}: This land becomes a 2/2 red and white Ox creature until end of turn. It's still a land." (CR 608.2, 611.2a).
fn land_becomes_2_2_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_2 = {
        let q: &dyn Query = g;
        Some(item.object)
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let kind_2_1_0 = {
        let q: &dyn Query = g;
        EffectKind::SetColors(q.words(item.object).colors(ColorSet(9)))
    };
    let kind_2_2_0 = {
        let q: &dyn Query = g;
        EffectKind::AddSubtype(q.words(item.object).subtype(Subtype::Ox))
    };
    let kind_2_3_0 = EffectKind::AddTypes(TypeSet::of(&[CardType::Creature]));
    instruction(
        g,
        |g| {
            for &object in &objects_2 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::SetPt(2, 2),
                    Duration::EndOfTurn,
                );
                g.add_effect(item.object, object, kind_2_1_0, Duration::EndOfTurn);
                g.add_effect(item.object, object, kind_2_2_0, Duration::EndOfTurn);
                g.add_effect(item.object, object, kind_2_3_0, Duration::EndOfTurn);
            }
        },
    );
    Ok(())
}

/// "Whenever this land attacks, put a +1/+1 counter on target creature you control." (CR 115.1).
fn put_1_1_counter_target_target_0(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Creature) && asker.controller == q.controller(id)))
    )
}

/// "Whenever this land attacks, put a +1/+1 counter on target creature you control." (CR 608.2, 122.1).
fn put_1_1_counter_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let mut counters_3: Vec<(ObjectId, CounterKind, i16)> = Vec::new();
    for t in t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.object())
        .into_iter()
        .map(Target::Object)
        .collect::<Vec<Target>>()
    {
        if let Target::Object(object) = t {
            counters_3.push((object, CounterKind::PlusOne, 1i16));
        }
    }
    g.add_counters_together(&counters_3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            TAP,
            add_one_of(&[Color::White, Color::Red]),
            "{T}: Add {R} or {W}.",
        ),
        ability(
            cost("{1}{W}{R}"),
            land_becomes_2_2_red,
            "{1}{R}{W}: This land becomes a 2/2 red and white Ox creature until end of turn. It's still a land.",
        ),
        triggered(
                triggers::THIS_ATTACKS,
                put_1_1_counter_target,
                "Whenever this land attacks, put a +1/+1 counter on target creature you control.",
            )
            .targets(&[target(put_1_1_counter_target_target_0)]),
    ],
    enters: Some(add_r_w_1_r_enters),
    ..oracle::RESTLESS_BIVOUAC
};
