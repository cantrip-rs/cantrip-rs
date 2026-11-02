use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At the beginning of combat on your turn, if you control an artifact, this creature gets +2/+1 until end of turn. If you control three or more artifacts, it also gains first strike until end of turn." (CR 608.2).
fn if_control_artifact_creature_gets_if(q: &dyn Query, asker: &Asker) -> bool {
    q
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            q.controller(id) == Some(asker.controller).unwrap_or(PlayerId(u8::MAX))
                && q.chars(id).is(CardType::Artifact)
        })
        .count() >= 1
}

/// "At the beginning of combat on your turn, if you control an artifact, this creature gets +2/+1 until end of turn. If you control three or more artifacts, it also gains first strike until end of turn." (CR 608.2, 611.2a).
fn if_control_artifact_creature_gets(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut hoisted_2: Vec<ObjectId> = Vec::new();
    let objects_0 = {
        let q: &dyn Query = g;
        Some(item.object)
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::Pump(2, 1),
                    Duration::EndOfTurn,
                );
            }
        },
    );
    g.end_sentence();
    if {
        let q: &dyn Query = g;
        (q
            .state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).is(CardType::Artifact) && item.controller == q.controller(id)
            })
            .collect::<Vec<ObjectId>>()
            .len() as i64) >= 3
    } {
        let objects_1 = {
            let q: &dyn Query = g;
            Some(item.object)
                .into_iter()
                .filter(|&o| q.state().obj(o).became.is_none())
                .collect::<Vec<ObjectId>>()
        };
        let kind_1_0_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::FirstStrike]));
        instruction(
            g,
            |g| {
                for &object in &objects_1 {
                    g.add_effect(item.object, object, kind_1_0_0, Duration::EndOfTurn);
                }
            },
        );
        g.end_sentence();
        hoisted_2.extend(objects_1.clone());
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                triggers::BEGINNING_OF_YOUR_COMBAT,
                if_control_artifact_creature_gets,
                "At the beginning of combat on your turn, if you control an artifact, this creature gets +2/+1 until end of turn. If you control three or more artifacts, it also gains first strike until end of turn.",
            )
            .condition(if_control_artifact_creature_gets_if),
    ],
    ..oracle::TOOLCRAFT_EXEMPLAR
};
