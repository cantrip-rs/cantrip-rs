use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Prevent all damage that would be dealt to this creature by creatures it's blocking." (CR 615.1).
fn prevent_all_damage_would_be_applies(
    q: &dyn Query,
    asker: &Asker,
    e: &DamageEvent,
) -> bool {
    let (source, target) = (e.source, &e.target);
    q.chars(source).is(CardType::Creature)
        && q.creatures_blocked_by(asker.source).contains(&source)
        && matches!(*target, Target::Object(o) if asker.source == o)
}

pub static CARD: CardDef = CardDef {
    preventions: &[
        PreventionDef {
            hint: "prevents all damage that would be dealt to this creature by creatures it's blocking",
            applies: prevent_all_damage_would_be_applies,
            amount: PreventionAmount::All,
            then: None,
        },
    ],
    ..oracle::WALL_OF_VAPOR
};
