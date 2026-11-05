use cantrip_api::*;
use cantrip_mechanics::cumulative_upkeep;
use cantrip_oracle as oracle;

/// "Cast this spell only if you control a snow land." (CR 601.3).
fn cast_spell_only_if_control_if(q: &dyn Query, asker: &Asker) -> bool {
    q.state().battlefield().iter().copied().any(|id| {
        let chars = q.chars(id);
        q.controller(id) == asker.controller && chars.supertypes.snow && chars.is(CardType::Land)
    })
}

/// "Creatures with flying don't untap during their controllers' untap steps." (CR 604.1).
fn creatures_flying_dont_untap_during_rule(
    q: &dyn Query,
    _: &Asker,
    id: ObjectId,
) -> bool {
    q.chars(id).is(CardType::Creature) && q.chars(id).keywords.has(Keyword::Flying)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(
            cumulative_upkeep::Payment::Mana(mana("{2}")),
            "Cumulative upkeep"
        ),
    ],
    untap_bans: &[creatures_flying_dont_untap_during_rule],
    cast_only_if: Some(cast_spell_only_if_control_if),
    ..oracle::BLIZZARD
};
