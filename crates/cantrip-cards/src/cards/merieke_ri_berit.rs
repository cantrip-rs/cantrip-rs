use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "When Merieke Ri Berit leaves the battlefield or becomes untapped" (CR 603.2).
fn delayed_2_event(_: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    if let Event::ZoneChange { old, from, .. } = event {
        let id = *old;
        let subject = Target::Object(id);
        if from.is(ZoneKind::Battlefield) && asker.source == id {
            return Fired::about(subject);
        }
    }
    let Event::Untapped { permanent } = event else { return None };
    let id = *permanent;
    let subject = Target::Object(id);
    if asker.source == id {
        return Fired::about(subject);
    }
    None
}

/// "When Merieke Ri Berit leaves the battlefield or becomes untapped, destroy that creature." (CR 608.2, 701.8a).
fn delayed_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_3 = g.event_objects(item).to_vec();
    g.destroy_all_no_regen(&objects_3);
    Ok(())
}

/// "{T}: Gain control of target creature for as long as you control Merieke Ri Berit. When Merieke Ri Berit leaves the battlefield or becomes untapped, destroy that creature. It can't be regenerated." (CR 608.2, 611.2a).
fn gain_control_target_creature_as(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    effects::gain_control_of_target_while_you_control_source(g, item)?;
    g.end_sentence();
    let about: Vec<ObjectId> = t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.object())
        .into_iter()
        .collect::<Vec<ObjectId>>();
    g.delayed_trigger_about(
        DelayedWhen::WhenEvent,
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        &about,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::DoesNotUntap]),
    abilities: &[
        ability(
                TAP,
                gain_control_target_creature_as,
                "{T}: Gain control of target creature for as long as you control Merieke Ri Berit. When Merieke Ri Berit leaves the battlefield or becomes untapped, destroy that creature. It can't be regenerated.",
            )
            .targets(&[targets::CREATURE]),
    ],
    delayed: &[
        triggered(
            when_asked(
                EventMask(EventMask::ZONE_CHANGE.0 | EventMask::UNTAPPED.0),
                delayed_2_event,
            ),
            delayed_2,
            "When Merieke Ri Berit leaves the battlefield or becomes untapped, destroy that creature.",
        ),
    ],
    ..oracle::MERIEKE_RI_BERIT
};
