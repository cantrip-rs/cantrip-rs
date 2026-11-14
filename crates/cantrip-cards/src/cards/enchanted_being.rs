use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Prevent all combat damage that would be dealt to this creature by enchanted creatures." (CR 615.1).
fn prevent_all_combat_damage_would_applies(
    q: &dyn Query,
    asker: &Asker,
    e: &DamageEvent,
) -> bool {
    {
        let (source, target, combat) = (e.source, &e.target, e.combat);
        if combat
            && q.chars(source).is(CardType::Creature)
            && q.battlefield_source_is_enchanted(source)
            && matches!(* target, Target::Object(o) if asker.source == o)
        {
            return true;
        }
    }
    false
}

pub static CARD: CardDef = CardDef {
    preventions: &[
        PreventionDef {
            hint: "prevents all combat damage that would be dealt to this creature by enchanted creatures",
            applies: prevent_all_combat_damage_would_applies,
            amount: PreventionAmount::All,
        },
    ],
    ..oracle::ENCHANTED_BEING
};
