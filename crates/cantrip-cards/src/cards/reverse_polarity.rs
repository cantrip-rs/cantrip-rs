use cantrip_api::*;
use cantrip_oracle as oracle;

/// Gain twice the damage dealt to you this turn by artifact permanents (CR 109.2, 119.3).
fn gain_x_life_where_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let q: &dyn Query = g;
    let damage: i128 = q.history().damage_dealt()
        .filter(|d| d.target == Target::Player(item.controller)
            && matches!(q.state().obj(d.source).zone.kind(), ZoneKind::Battlefield | ZoneKind::PhasedOut)
            && d.chars(q).is(CardType::Artifact))
        .map(|d| i128::from(d.amount))
        .sum();
    g.gain_life(item.controller, (damage * 2).clamp(0, i128::from(i32::MAX)) as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], gain_x_life_where_x),
    ..oracle::REVERSE_POLARITY
};
