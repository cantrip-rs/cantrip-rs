use cantrip_api::*;
use cantrip_mechanics::replacements;
use cantrip_oracle as oracle;

/// "{T}: All combat damage that would be dealt to you by unblocked creatures this turn is dealt to this creature instead." (CR 614.1a).
fn made_1_applies(
    q: &dyn Query,
    asker: &Asker,
    _: ObjectId,
    proposal: &Proposal,
) -> bool {
    let Proposal::Damage(e) = proposal else { return false };
    let (source, target, combat) = (e.source, &e.target, e.combat);
    if combat && !q.state().obj(source).zone.is(ZoneKind::Stack)
        && (q.state().attackers.iter().any(|a| a.id == source && !a.blocked)
            && q.chars(source).is(CardType::Creature))
        && matches!(* target, Target::Player(p) if asker.controller == p)
    {
        return true;
    }
    false
}

/// "{T}: All combat damage that would be dealt to you by unblocked creatures this turn is dealt to this creature instead." (CR 614.1a).
fn made_1_apply(q: &dyn Query, asker: &Asker, _: ObjectId, proposal: &mut Proposal) {
    replacements::dealt_to_this_creature_instead(q, asker, proposal);
}

/// "{T}: All combat damage that would be dealt to you by unblocked creatures this turn is dealt to this creature instead." (CR 608.2).
fn all_combat_damage_would_be(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    g.add_player_effect(
        item.controller,
        item.object,
        EffectKind::ReplacementRule { card: item.ability.card, index: 0 },
        Duration::EndOfTurn,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            TAP,
            all_combat_damage_would_be,
            "{T}: All combat damage that would be dealt to you by unblocked creatures this turn is dealt to this creature instead.",
        ),
    ],
    granted_replacements: &[
        granted_replacement(ReplacementKind::Damage, made_1_applies, made_1_apply, "")
            .text(
                "All combat damage that would be dealt to you by unblocked creatures this turn is dealt to this creature instead.",
            ),
    ],
    ..oracle::KJELDORAN_ROYAL_GUARD
};
