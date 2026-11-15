use cantrip_api::*;
use cantrip_mechanics::{damage_target, effects, permanents_with_subtype_you_control, targets};
use cantrip_oracle as oracle;

/// "Target attacking, blocking, or tapped creature" (CR 115.1, 506.4, 508.1, 509.1, 701.26): a creature on the battlefield that is
/// attacking or blocking now, or tapped now, judged as the spell is cast and again as it resolves (CR 608.2b).
fn attacking_blocking_or_tapped_creature(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::attacking_or_blocking_creature_filter(q, asker, t)
        || (targets::creature_filter(q, asker, t) && matches!(t, Target::Object(id) if q.obj(id).tapped))
}

/// "Sonar Strike deals 4 damage to target attacking, blocking, or tapped creature. You gain 3 life if you control a Bat." (CR 120.3,
/// 119.3, 608.2c): the damage, then, as a second instruction, the Bat check as it is followed, read through the layers and by the
/// word the spell's text says now (CR 612.2). A target no longer legal means the spell does not resolve at all, so no life is
/// gained even with a Bat (CR 608.2b, the ruling).
fn sonar_strike(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 4);
    g.end_sentence();
    let bat = g.words(item.object).subtype(Subtype::Bat);
    if permanents_with_subtype_you_control(g, item.controller, bat) > 0 {
        effects::gain_life(g, item, 3)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[target(attacking_blocking_or_tapped_creature)], sonar_strike), ..oracle::SONAR_STRIKE };
