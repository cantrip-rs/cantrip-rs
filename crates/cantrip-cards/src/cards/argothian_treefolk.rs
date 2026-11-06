use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Prevent all damage that would be dealt to this creature by artifact sources." (CR 615.1).
fn prevent_all_damage_would_be_applies(
    q: &dyn Query,
    asker: &Asker,
    e: &DamageEvent,
) -> bool {
    {
        let (source, target) = (e.source, &e.target);
        if q.chars(source).is(CardType::Artifact)
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
            hint: "prevents all damage that would be dealt to this creature by artifact sources",
            applies: prevent_all_damage_would_be_applies,
            amount: PreventionAmount::All,
        },
    ],
    ..oracle::ARGOTHIAN_TREEFOLK
};
