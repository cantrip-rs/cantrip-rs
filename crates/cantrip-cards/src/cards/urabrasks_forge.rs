use cantrip_api::*;
use cantrip_oracle as oracle;

/// "At the beginning of combat on your turn" (CR 603.2).
fn when_beginning_combat(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::StepBegan { step } = event else { return None };
    let subject = Target::Player(q.state().turn.active);
    if matches!(* step, Step::BeginCombat) && asker.controller == q.state().turn.active {
        return Fired::about(subject);
    }
    None
}

pub static PHYREXIAN_HORROR: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Creature]),
    colors: Some(ColorSet(8)),
    power: Some(0),
    toughness: Some(0),
    keywords: Keywords::of(&[Keyword::Trample, Keyword::Haste]),
    ..token!(Phyrexian Horror)
};

/// "At the beginning of combat on your turn, put an oil counter on this artifact, then create an X/1 red Phyrexian Horror creature token with trample and haste, where X is the number of oil counters on this artifact. Sacrifice that token at the beginning of the next end step." (CR 608.2, 701.21a).
fn delayed_4(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_5 = {
        let q: &dyn Query = g;
        about(q, item, 0)
    };
    let players_5 = Some(item.controller).into_iter().collect::<Vec<PlayerId>>();
    let mut picks_5: Vec<(PlayerId, ObjectId)> = Vec::new();
    for &object in &objects_5 {
        let controller = g.controller(object);
        if players_5.contains(&controller) && g.can_sacrifice(controller, object) {
            picks_5.push((controller, object));
        }
    }
    g.sacrifice_each(&picks_5);
    Ok(())
}

/// "At the beginning of combat on your turn, put an oil counter on this artifact, then create an X/1 red Phyrexian Horror creature token with trample and haste, where X is the number of oil counters on this artifact. Sacrifice that token at the beginning of the next end step." (CR 608.2, 111.1, 122.1).
fn put_oil_counter_artifact_create(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut counters_0: Vec<(ObjectId, CounterKind, i16)> = Vec::new();
    for t in Some(item.object).into_iter().map(Target::Object).collect::<Vec<Target>>() {
        if let Target::Object(object) = t {
            counters_0.push((object, CounterKind::Oil, 1i16));
        }
    }
    g.add_counters_together(&counters_0);
    g.end_sentence();
    let power_1 = {
        let q: &dyn Query = g;
        Some(item.object)
            .into_iter()
            .map(|o: ObjectId| i64::from(q.state().obj(o).counters(CounterKind::Oil)))
            .sum::<i64>() as i16
    };
    let mut values_1 = Copiable::of(g.card_id(&PHYREXIAN_HORROR)).sized(power_1, 1);
    values_1.words = g.words(item.object);
    let mut created_1: Vec<ObjectId> = Vec::new();
    created_1.extend(g.create_token_copies_tapped(values_1, item.controller, 1, false)?);
    g.end_sentence();
    {
        let about: Vec<ObjectId> = created_1.clone();
        {
            g.delayed_trigger_about(
                DelayedWhen::Step {
                    step: Step::End,
                    on_turn_of: None,
                },
                item.object,
                item.controller,
                AbilityRef::delayed(item.ability.card, 0),
                &about,
            );
        }
    }
    Ok(())
}

/// "At the beginning of combat on your turn, put an oil counter on this artifact, then create an X/1 red Phyrexian Horror creature token with trample and haste, where X is the number of oil counters on this artifact. Sacrifice that token at the beginning of the next end step." (CR 608.2).
fn about(q: &dyn Query, item: &StackItem, part: usize) -> Vec<ObjectId> {
    q.event_objects(item)
        .split(|&id| id == ObjectId::NONE)
        .nth(part)
        .map_or_else(Vec::new, <[ObjectId]>::to_vec)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::STEP_BEGAN, when_beginning_combat),
            put_oil_counter_artifact_create,
            "At the beginning of combat on your turn, put an oil counter on this artifact, then create an X/1 red Phyrexian Horror creature token with trample and haste, where X is the number of oil counters on this artifact. Sacrifice that token at the beginning of the next end step.",
        ),
    ],
    delayed: &[
        delayed_trigger(
            delayed_4,
            "Sacrifice that token at the beginning of the next end step.",
        ),
    ],
    ..oracle::URABRASKS_FORGE
};

pub static EXTRA: &[&CardDef] = &[&PHYREXIAN_HORROR];
