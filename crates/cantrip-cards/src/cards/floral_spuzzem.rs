use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks and isn't blocked, you may destroy target artifact defending player controls. If you do, this creature assigns no combat damage this turn." (CR 115.1).
fn destroy_target_artifact_defending_player_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Artifact) && Some(asker.source).and_then(| o : ObjectId |
        q.defending_player(o)) == Some(q.controller(id))))
    )
}

/// "Whenever this creature attacks and isn't blocked, you may destroy target artifact defending player controls. If you do, this creature assigns no combat damage this turn." (CR 118.12, 611.2a, 701.8a).
fn destroy_target_artifact_defending_player(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    if effects::ask_may(g, item.controller, item.object, Purpose::Destroy)? {
        cantrip_mechanics::destroy_target(g, item, 0);
        g.end_sentence();
        g.add_effect(item.object, item.object, EffectKind::NoCombatDamageAssignment, Duration::EndOfTurn);
    }

    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                when_asked(EventMask::UNBLOCKED, triggers::this_attacks_and_isnt_blocked),
                destroy_target_artifact_defending_player,
                "Whenever this creature attacks and isn't blocked, you may destroy target artifact defending player controls. If you do, this creature assigns no combat damage this turn.",
            )
            .targets(&[target(destroy_target_artifact_defending_player_target_0)]),
    ],
    ..oracle::FLORAL_SPUZZEM
};
