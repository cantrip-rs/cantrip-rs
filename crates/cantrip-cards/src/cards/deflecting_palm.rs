use cantrip_api::*;
use cantrip_mechanics::prevention;
use cantrip_oracle as oracle;

/// "The next time a source of your choice would deal damage to you this turn, prevent that damage" (CR 609.7a, 615.8): the
/// source is chosen as the spell resolves; it is not targeted.
fn palm(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    prevention::next_time_chosen_source_would_damage_you(g, item, 0)?;
    Ok(())
}

/// "If damage is prevented this way, Deflecting Palm deals that much damage to that source's controller" (CR 615.5): the
/// controller of the object that would have dealt the damage, as it last existed (CR 608.2h) - the chosen source, or the
/// permanent a chosen permanent spell became (CR 609.7a) - the damage the Palm itself deals.
fn deflect(g: &mut dyn Game, asker: &Asker, _chosen: ObjectId, prevented: &[Prevented]) {
    let Some(first) = prevented.first() else { return };
    let to = g.controller(first.source);
    let amount = Prevented::total(prevented);
    g.damage(asker.source, Target::Player(to), i64::from(amount));
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], palm),
    granted_preventions: &[GrantedPreventionDef {
        then: Some(deflect),
        text: "The next time a source of your choice would deal damage to you this turn, prevent that damage. If damage is prevented this way, Deflecting Palm deals that much damage to that source's controller.",
        ..prevention::FROM_CHOSEN_SOURCE
    }],
    ..oracle::DEFLECTING_PALM
};
