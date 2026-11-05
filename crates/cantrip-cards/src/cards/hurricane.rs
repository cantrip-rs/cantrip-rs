use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Deal damage simultaneously to each player and each creature with flying (CR 101.4, 608.2h).
fn hurricane(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut recipients: Vec<Target> = g.state().battlefield().iter().copied()
        .filter(|&id| targets::has_flying_creature(g, id))
        .map(Target::Object).collect();
    recipients.extend(g.state().apnap().map(Target::Player));
    let damage: Vec<_> = recipients.into_iter().map(|target| (item.object, target, i64::from(item.x))).collect();
    g.damage_together(&damage);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], hurricane),
    ..oracle::HURRICANE
};
