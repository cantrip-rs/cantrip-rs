use cantrip_api::*;
use cantrip_mechanics::protection;
use cantrip_oracle as oracle;

/// "Prevent all damage that would be dealt to this creature by enchanted creatures." (CR 615.1).
fn prevent_all_damage_would_be_applies(
    q: &dyn Query,
    asker: &Asker,
    e: &DamageEvent,
) -> bool {
    matches!(e.target, Target::Object(o) if asker.source == o)
        && q.chars(e.source).is(CardType::Creature)
        && q.battlefield_source_is_enchanted(e.source)
}

pub static CARD: CardDef = CardDef {
    statics: &[protection::from::<{ Color::White as u8 }>("Protection from white")],
    preventions: &[
        PreventionDef {
            hint: "prevents all damage that would be dealt to this creature by enchanted creatures",
            applies: prevent_all_damage_would_be_applies,
            amount: PreventionAmount::All,
        },
    ],
    ..oracle::WALL_OF_PUTRID_FLESH
};
