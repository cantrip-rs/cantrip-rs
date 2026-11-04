use cantrip_api::*;
use cantrip_mechanics::effects::target_player_loses_1_life_you_gain_1;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Whenever this creature or another creature or artifact you control dies" (CR 700.4, 603.10a): any creature or artifact
/// that was under this controller's control as it last was on the battlefield, Salvagers itself among them, looked back on, so
/// several dying together trigger it once each (the ruling). An artifact creature is one event and triggers once.
fn creature_or_artifact_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if from.is(ZoneKind::Battlefield)
                && to.is(ZoneKind::Graveyard)
                && q.obj(*old).controller == me.controller
                && (q.chars(*old).is(CardType::Creature) || q.chars(*old).is(CardType::Artifact)) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, creature_or_artifact_you_control_dies),
        target_player_loses_1_life_you_gain_1,
        "Whenever this creature or another creature or artifact you control dies, target opponent loses 1 life and you gain 1 life.",
    )
    .targets(&[targets::OPPONENT])],
    ..oracle::AL_BHED_SALVAGERS
};
