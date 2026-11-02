use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "This land enters tapped. {T}: Add {R} or {G}. {2}{R}{G}: This land becomes a 3/4 red and green Dinosaur creature until end of turn. It's still a land. Whenever this land attacks, another target attacking creature gets +2/+0 until end of turn. Untap that creature." (CR 608.2, 701.26a).
fn add_r_g_2_r_enters(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.tapped = true;
    Ok(())
}

/// "{2}{R}{G}: This land becomes a 3/4 red and green Dinosaur creature until end of turn. It's still a land." (CR 608.2, 611.2a).
fn land_becomes_3_4_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_2 = {
        let q: &dyn Query = g;
        Some(item.object)
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let kind_2_1_0 = {
        let q: &dyn Query = g;
        EffectKind::SetColors(q.words(item.object).colors(ColorSet(24)))
    };
    let kind_2_2_0 = {
        let q: &dyn Query = g;
        EffectKind::AddSubtype(q.words(item.object).subtype(Subtype::Dinosaur))
    };
    let kind_2_3_0 = EffectKind::AddTypes(TypeSet::of(&[CardType::Creature]));
    instruction(
        g,
        |g| {
            for &object in &objects_2 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::SetPt(3, 4),
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

/// "Whenever this land attacks, another target attacking creature gets +2/+0 until end of turn. Untap that creature." (CR 115.1).
fn another_target_attacking_creature_gets_target_0(
    q: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((! (q
        .event_subject().object() == Some(id)) && ((q.state().attackers.iter().any(| a |
        a.id == id) || q.was_attacking(id)) && q.chars(id).is(CardType::Creature))))
    )
}

/// "Whenever this land attacks" (CR 603.2).
fn when_land_attacks(_: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::Attacks { attacker, .. } = event else { return None };
    let id = *attacker;
    let subject = Target::Object(id);
    if asker.source == id {
        return Fired::about(subject);
    }
    None
}

/// "Whenever this land attacks, another target attacking creature gets +2/+0 until end of turn. Untap that creature." (CR 608.2, 611.2a, 701.26a).
fn another_target_attacking_creature_gets(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_3 = targets::object(g, item, 0);
    instruction(
        g,
        |g| {
            if let Some(object) = objects_3 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::Pump(2, 0),
                    Duration::EndOfTurn,
                );
            }
        },
    );
    g.end_sentence();
    let objects_4 = {
        let q: &dyn Query = g;
        t_0.get(0)
            .copied()
            .and_then(|t: Target| t.object())
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let turned_4: Vec<ObjectId> = objects_4
        .iter()
        .copied()
        .filter(|&o| g.state().obj(o).tapped != false)
        .collect();
    instruction(
        g,
        |g| {
            for &object in &turned_4 {
                g.untap(object);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            TAP,
            add_one_of(&[Color::Red, Color::Green]),
            "{T}: Add {R} or {G}.",
        ),
        ability(
            cost("{2}{R}{G}"),
            land_becomes_3_4_red,
            "{2}{R}{G}: This land becomes a 3/4 red and green Dinosaur creature until end of turn. It's still a land.",
        ),
        triggered(
                when_asked(EventMask::ATTACKS, when_land_attacks),
                another_target_attacking_creature_gets,
                "Whenever this land attacks, another target attacking creature gets +2/+0 until end of turn. Untap that creature.",
            )
            .targets(&[target(another_target_attacking_creature_gets_target_0)]),
    ],
    enters: Some(add_r_g_2_r_enters),
    ..oracle::RESTLESS_RIDGELINE
};
