use cantrip_api::*;
use cantrip_oracle as oracle;

/// "As long as this creature is attacking, prevent all damage Deserts would deal to this creature and to creatures banded with this creature." (CR 615.1).
fn as_long_as_creature_attacking_applies(
    q: &dyn Query,
    asker: &Asker,
    e: &DamageEvent,
) -> bool {
    {
        let (source, target) = (e.source, &e.target);
        if !q.state().obj(source).zone.is(ZoneKind::Stack)
            && q
                .chars(source)
                .has_subtype(q.words(asker.source).subtype(Subtype::Desert))
            && matches!(
                * target, Target::Object(o) if asker.source == o || (q.chars(o)
                .is(CardType::Creature) && Some(asker.source).is_some_and(| other :
                ObjectId | { let band = | o : ObjectId | q.state().attackers.iter()
                .find(| a | a.id == o).map_or(0, | a | a.band); o != other && band(o) !=
                0 && band(o) == band(other) }))
            )
            && q.is_attacking(asker.source)
        {
            return true;
        }
    }
    false
}

pub static CARD: CardDef = CardDef {
    preventions: &[
        PreventionDef {
            hint: "As long as this creature is attacking, prevent all damage Deserts would deal to this creature and to creatures banded with this creature.",
            applies: as_long_as_creature_attacking_applies,
            amount: PreventionAmount::All,
        },
    ],
    ..oracle::CAMEL
};
