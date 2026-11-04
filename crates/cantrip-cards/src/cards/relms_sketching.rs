use cantrip_api::*;
use cantrip_mechanics::copy;
use cantrip_oracle as oracle;

/// "Target artifact, creature, or land" (CR 115.1, 205.2): a permanent on the battlefield, of any controller, that is one of the
/// three as the layers have it now.
fn artifact_creature_or_land(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.obj(id).zone.is(ZoneKind::Battlefield) && {
        let c = q.chars(id);
        c.is(CardType::Artifact) || c.is(CardType::Creature) || c.is(CardType::Land)
    })
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(artifact_creature_or_land)], copy::create_token_copy_of_target),
    ..oracle::RELMS_SKETCHING
};

