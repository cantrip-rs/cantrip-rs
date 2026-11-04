use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "All damage that would be dealt this turn by target sorcery spell is dealt to that spell's controller instead." (CR 115.1).
fn all_damage_would_be_dealt_target_0(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::SPELL.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if q.chars(id).is(CardType::Sorcery))
}

/// "All damage that would be dealt this turn by target sorcery spell is dealt to that spell's controller instead." (CR 614.1a).
fn made_1_applies(
    q: &dyn Query,
    _: &Asker,
    affected: ObjectId,
    proposal: &Proposal,
) -> bool {
    let Proposal::Damage(e) = proposal else { return false };
    let source = e.source;
    if q.state().obj(source).zone.is(ZoneKind::Stack) && affected == source {
        return true;
    }
    false
}

/// "All damage that would be dealt this turn by target sorcery spell is dealt to that spell's controller instead." (CR 614.1a).
fn made_1_apply(q: &dyn Query, _: &Asker, _: ObjectId, proposal: &mut Proposal) {
    if let Proposal::Damage(e) = proposal {
        e.target = Target::Player(q.controller(e.source));
    }
}

/// "All damage that would be dealt this turn by target sorcery spell is dealt to that spell's controller instead." (CR 608.2, 611.2a).
fn all_damage_would_be_dealt(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_effect(item.object, spell, EffectKind::Replacement { card: item.ability.card, index: 0 }, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[target(all_damage_would_be_dealt_target_0)],
        all_damage_would_be_dealt,
    ),
    granted_replacements: &[
        granted_replacement(
                ReplacementKind::Damage,
                made_1_applies,
                made_1_apply,
                "All damage that would be dealt this turn by target sorcery spell is dealt to that spell's controller instead.",
            )
            .text(
                "All damage that would be dealt this turn by target sorcery spell is dealt to that spell's controller instead.",
            ),
    ],
    ..oracle::REVERBERATION
};
