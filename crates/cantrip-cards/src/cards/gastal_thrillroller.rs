use cantrip_api::*;
use cantrip_mechanics::crew;
use cantrip_oracle as oracle;

/// "When this Vehicle enters" (CR 603.2).
fn when_vehicle_enters(_: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { new, to, .. } = event else { return None };
    let id = *new;
    let subject = Target::Object(id);
    if to.is(ZoneKind::Battlefield) && asker.source == id {
        return Fired::about(subject);
    }
    None
}

/// "When this Vehicle enters, it becomes an artifact creature until end of turn." (CR 608.2, 611.2a).
fn becomes_artifact_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        item.subject
            .object()
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let kind_0_0_0 = EffectKind::AddTypes(TypeSet::of(&[CardType::Artifact]));
    let kind_0_1_0 = EffectKind::AddTypes(TypeSet::of(&[CardType::Creature]));
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(item.object, object, kind_0_0_0, Duration::EndOfTurn);
                g.add_effect(item.object, object, kind_0_1_0, Duration::EndOfTurn);
            }
        },
    );
    Ok(())
}

/// "{2}{R}, Discard a card: Return this card from your graveyard to the battlefield with a finality counter on it. Activate only as a sorcery." (CR 608.2).
fn return_card_graveyard_battlefield_finality(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_1 = Some(item.object)
        .into_iter()
        .filter(|&o: &ObjectId| !g.state().obj(o).token)
        .collect::<Vec<ObjectId>>();
    {
        let rows_1: Vec<(ObjectId, PlayerId, bool)> = objects_1
            .iter()
            .map(|&o| (o, item.controller, false))
            .collect();
        let counters = [(CounterKind::Finality, 1i16)];
        g.put_all_onto_battlefield_with_counters(&rows_1, &counters)?
    };
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, when_vehicle_enters),
            becomes_artifact_creature,
            "When this Vehicle enters, it becomes an artifact creature until end of turn.",
        ),
        crew::crew(2, "Crew 2"),
        ability(
                cost("{2}{R}").and(discard_cards(1)),
                return_card_graveyard_battlefield_finality,
                "{2}{R}, Discard a card: Return this card from your graveyard to the battlefield with a finality counter on it. Activate only as a sorcery.",
            )
            .sorcery_speed()
            .from(ZoneKind::Graveyard),
    ],
    ..oracle::GASTAL_THRILLROLLER
};
