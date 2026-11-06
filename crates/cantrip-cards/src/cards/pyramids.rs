use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{2}: Choose one — • Destroy target Aura attached to a land. • The next time target land would be destroyed this turn, remove all damage marked on it instead." (CR 115.1).
fn choose_one_destroy_target_aura_mode_0_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if targets::permanent(q, id)
            && q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Aura))
            && q.state().obj(id).attached_to.is_some_and(|host|
                targets::permanent(q, host) && q.chars(host).is(CardType::Land))
    )
}

/// "{2}: Choose one — • Destroy target Aura attached to a land. • The next time target land would be destroyed this turn, remove all damage marked on it instead." (CR 614.1a).
fn made_3_applies(
    q: &dyn Query,
    _: &Asker,
    affected: ObjectId,
    proposal: &Proposal,
) -> bool {
    let Proposal::ZoneChange(z) = proposal else { return false };
    let id = z.object;
    if z.destruction() && affected == id && z.to == Zone::graveyard(q.obj(id).owner) {
        return true;
    }
    false
}

/// "{2}: Choose one — • Destroy target Aura attached to a land. • The next time target land would be destroyed this turn, remove all damage marked on it instead." (CR 614.1a).
fn made_3_apply(_: &dyn Query, _: &Asker, _: ObjectId, proposal: &mut Proposal) {
    let seen = *proposal;
    let Proposal::ZoneChange(_) = &seen else { return };
    let Proposal::ZoneChange(z) = proposal else { return };
    z.replaced = true;
}

/// "{2}: Choose one — • Destroy target Aura attached to a land. • The next time target land would be destroyed this turn, remove all damage marked on it instead." (CR 614.1a).
fn made_3_then(
    g: &mut dyn Game,
    _: &Asker,
    affected: ObjectId,
    _: &Proposal,
    _: &Proposal,
) {
    g.heal(affected);
}

/// "{2}: Choose one — • Destroy target Aura attached to a land. • The next time target land would be destroyed this turn, remove all damage marked on it instead." (CR 608.2, 611.2a).
fn choose_one_destroy_target_aura_mode_1(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    {
        let to = Some(Target::None);
        let about = t_0.get(0).copied().and_then(|t: Target| t.object());
        if let (Some(to), Some(about)) = (to, about) {
            let _ = to;
            g.add_effect(
                item.object,
                about,
                EffectKind::Replacement {
                    card: item.ability.card,
                    index: 0,
                },
                Duration::EndOfTurn,
            );
        }
    }
    Ok(())
}

static ABILITY_0_MODES: ModalDef = modes(
        &[
            mode(
                    effects::destroy_first_target,
                    "Destroy target Aura attached to a land.",
                )
                .targets(&[target(choose_one_destroy_target_aura_mode_0_target_0)]),
            mode(
                    choose_one_destroy_target_aura_mode_1,
                    "The next time target land would be destroyed this turn, remove all damage marked on it instead.",
                )
                .targets(&[targets::LAND]),
        ],
    )
    .choose(1, 1);

pub static CARD: CardDef = CardDef {
    abilities: &[
        modal_ability(
            cost("{2}"),
            &ABILITY_0_MODES,
            "{2}: Choose one — Destroy target Aura attached to a land; or The next time target land would be destroyed this turn, remove all damage marked on it instead.",
        ),
    ],
    granted_replacements: &[
        granted_replacement(
                ReplacementKind::ZoneChange,
                made_3_applies,
                made_3_apply,
                "{2}: Choose one — Destroy target Aura attached to a land; or The next time target land would be destroyed this turn, remove all damage marked on it instead.",
            )
            .once()
            .then(made_3_then)
            .text(
                "The next time target land would be destroyed this turn, remove all damage marked on it instead.",
            ),
    ],
    ..oracle::PYRAMIDS
};
