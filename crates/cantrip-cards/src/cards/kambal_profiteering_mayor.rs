use cantrip_api::*;
use cantrip_mechanics::triggers::token_you_control_enters;
use cantrip_mechanics::effects::each_opponent_loses_1_life_you_gain_1;
use cantrip_oracle as oracle;

/// "Whenever one or more tokens your opponents control enter" (CR 603.6a, 603.2c): a token under an opponent's control as it
/// entered, however it was made. The token is the event's subject, so the batch that entered together is the subjects of the
/// matching events.
fn token_an_opponent_controls_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if to.is(ZoneKind::Battlefield)
                && q.obj(*new).token
                && q.state().opponents(me.controller).any(|o| o == q.controller(*new)) =>
        {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

/// "For each of them, create a tapped token that's a copy of it" (CR 707.2, 111.1, 614.1c): one token for each token of the batch,
/// a copy of that token's copiable values (the rulings: its original characteristics, an X being 0, not what a copy effect or
/// counters made it), created together and tapped as they enter, under the controller of this ability.
fn copy_each_tapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let copies: Vec<(Copiable, PlayerId)> = g.event_objects(item).to_vec().into_iter().map(|token| (g.copiable_values(token), item.controller)).collect();
    g.create_tapped_token_copies_together(&copies)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, token_an_opponent_controls_enters),
            copy_each_tapped,
            "Whenever one or more tokens your opponents control enter, for each of them, create a tapped token that's a copy of it. This ability triggers only once each turn.",
        )
        .once_per_batch()
        .triggers_each_turn(1),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, token_you_control_enters),
            each_opponent_loses_1_life_you_gain_1,
            "Whenever one or more tokens you control enter, each opponent loses 1 life and you gain 1 life.",
        )
        .once_per_batch(),
    ],
    ..oracle::KAMBAL_PROFITEERING_MAYOR
};
