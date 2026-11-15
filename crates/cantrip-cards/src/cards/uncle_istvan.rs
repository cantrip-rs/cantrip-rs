use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Prevent all damage that would be dealt to this creature by creatures." (CR 615.1, 109.2, 608.2h): the source is a
/// creature if it is a creature on the battlefield, or was one there as it last existed (a sacrificed Mogg Fanatic). A creature
/// card in a hand or a graveyard is not a creature (a discarded Trumpeting Carnosaur), nor is a creature spell on the stack.
/// A departed object's row keeps the zone it left from.
fn damage_by_a_creature_to_this(q: &dyn Query, asker: &Asker, e: &DamageEvent) -> bool {
    let state = q.state();
    let zone = state.obj(e.source).zone;
    (zone.is(ZoneKind::Battlefield) || zone.is(ZoneKind::PhasedOut))
        && state.last_known(e.source).map_or_else(|| q.chars(e.source).is(CardType::Creature), |row| row.types.has(CardType::Creature))
        && matches!(e.target, Target::Object(o) if o == asker.source)
}

pub static CARD: CardDef = CardDef {
    preventions: &[
        PreventionDef {
            hint: "prevents all damage that would be dealt to this creature by creatures",
            applies: damage_by_a_creature_to_this,
            amount: PreventionAmount::All,
            then: None,
        },
    ],
    ..oracle::UNCLE_ISTVAN
};
