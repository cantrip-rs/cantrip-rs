use cantrip_api::*;
use cantrip_mechanics::cumulative_upkeep;
use cantrip_oracle as oracle;

/// "Prevent all damage that would be dealt by instant and sorcery spells. Creatures with flying don't untap during their controllers' untap steps." (CR 615.1).
fn prevent_all_damage_would_be_applies(
    q: &dyn Query,
    _: &Asker,
    e: &DamageEvent,
) -> bool {
    {
        let source = e.source;
        if q.state().obj(source).zone.is(ZoneKind::Stack)
            && (q.chars(source).is(CardType::Instant)
                || q.chars(source).is(CardType::Sorcery))
        {
            return true;
        }
    }
    false
}

/// "Prevent all damage that would be dealt by instant and sorcery spells. Creatures with flying don't untap during their controllers' untap steps." (CR 604.1).
fn prevent_all_damage_would_be_rule(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && q.chars(id).keywords.has(Keyword::Flying)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(
            cumulative_upkeep::Payment::Mana(mana("{1}")),
            "Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)"
        ),
    ],
    preventions: &[
        PreventionDef {
            hint: "prevents all damage that would be dealt by instant and sorcery spells",
            applies: prevent_all_damage_would_be_applies,
            amount: PreventionAmount::All,
        },
    ],
    untap_bans: &[prevent_all_damage_would_be_rule],
    ..oracle::ENERGY_STORM
};
