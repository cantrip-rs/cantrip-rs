use cantrip_api::*;
use cantrip_mechanics::{crew, triggers, scry};
use cantrip_oracle as oracle;

/// "When this Vehicle enters, scry 1." (CR 608.2).
fn scry_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    scry(g, item.controller, item.object, 1)?;
    Ok(())
}

/// "Tap three other untapped creatures you control: Until end of turn, this Vehicle becomes an artifact creature and gains flying. Put a +1/+1 counter on it." (CR 608.2, 611.2a, 122.1).
fn vehicle_becomes_artifact_creature_gains(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_1 = {
        let q: &dyn Query = g;
        Some(item.object)
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let kind_1_0_0 = EffectKind::AddTypes(
        TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    );
    let kind_1_1_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying]));
    instruction(
        g,
        |g| {
            for &object in &objects_1 {
                g.add_effect(item.object, object, kind_1_0_0, Duration::EndOfTurn);
                g.add_effect(item.object, object, kind_1_1_0, Duration::EndOfTurn);
            }
        },
    );
    g.end_sentence();
    let mut counters_2: Vec<(ObjectId, CounterKind, i16)> = Vec::new();
    for t in Some(item.object).into_iter().map(Target::Object).collect::<Vec<Target>>() {
        if let Target::Object(object) = t {
            counters_2.push((object, CounterKind::PlusOne, 1i16));
        }
    }
    g.add_counters_together(&counters_2);
    Ok(())
}

/// "Tap three other untapped creatures you control: Until end of turn, this Vehicle becomes an artifact creature and gains flying. Put a +1/+1 counter on it." (CR 608.2, 701.26a).
fn vehicle_becomes_artifact_creature_gains_2(
    q: &dyn Query,
    asker: &Asker,
    id: ObjectId,
) -> bool {
    !(asker.source == id)
        && (!(q.state().obj(id).tapped) && q.chars(id).is(CardType::Creature)
            && asker.controller == q.controller(id))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, scry_1, "When this Vehicle enters, scry 1."),
        ability(
            tap_untapped(3, vehicle_becomes_artifact_creature_gains_2),
            vehicle_becomes_artifact_creature_gains,
            "Tap three other untapped creatures you control: Until end of turn, this Vehicle becomes an artifact creature and gains flying. Put a +1/+1 counter on it.",
        ),
        crew::crew(1, "Crew 1"),
    ],
    ..oracle::VOYAGER_GLIDECAR
};
