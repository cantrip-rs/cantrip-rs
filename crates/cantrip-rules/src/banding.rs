//! Ordinary banding and "bands with other" (CR 702.22c, j-k).
//! The latter's named quality is independent of its holder; no extra object state.
use cantrip_api::*;

fn qualities(
    q: &dyn Query,
    abilities: [Option<AbilityRef>; cantrip_state::MAX_ABILITIES],
) -> impl Iterator<Item = BandQuality> + '_ {
    abilities.into_iter().flatten().filter_map(move |a| {
        let card = q.card(a.card);
        let def = if a.is_delayed() {
            card.delayed.get((a.index - AbilityRef::DELAYED) as usize)
        } else {
            card.abilities.get(a.index as usize)
        };
        match def {
            Some(AbilityDef::BandsWithOthers(bands)) => Some(bands.quality),
            _ => None,
        }
    })
}

pub(crate) fn can_lead(q: &dyn Query, holder: ObjectId) -> bool {
    let chars = q.chars(holder);
    chars.has(Keyword::Banding) || qualities(q, chars.abilities).any(|quality| quality(q, holder))
}

fn members(
    source: ObjectId,
    selected: &[ObjectId],
    extra: Option<ObjectId>,
) -> impl Iterator<Item = ObjectId> + '_ {
    std::iter::once(source)
        .chain(selected.iter().copied())
        .chain(extra)
}

/// Whether these required members can form a band with a provider among `providers`.
/// A provider in the unused candidates may complete a partial selection; final legality
/// asks with providers restricted to selected members. No temporary member list is needed.
fn possible(
    q: &dyn Query,
    source: ObjectId,
    selected: &[ObjectId],
    extra: Option<ObjectId>,
    providers: &[ObjectId],
) -> bool {
    if members(source, selected, extra)
        .filter(|&id| !q.chars(id).has(Keyword::Banding))
        .take(2)
        .count()
        <= 1
        && members(source, providers, None).any(|id| q.chars(id).has(Keyword::Banding))
    {
        return true;
    }
    members(source, providers, None).any(|provider| {
        qualities(q, q.chars(provider).abilities).any(|quality| {
            quality(q, provider) && members(source, selected, extra).all(|id| quality(q, id))
        })
    })
}

pub(crate) fn selection(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict {
        complete: choice.selected.is_empty()
            || possible(q, choice.source, &choice.selected, None, &choice.selected),
        addable: Bits::of(choice.candidates.len(), |i| {
            possible(
                q,
                choice.source,
                &choice.selected,
                Some(choice.candidates[i]),
                &choice.candidates,
            )
        }),
    }
}

/// Unlike band formation, assignment control needs only a qualifying provider and
/// one OTHER member of that quality; unrelated blockers/attackers may also be present.
pub(crate) fn controls_assignment(q: &dyn Query, group: &[ObjectId]) -> bool {
    group.iter().any(|&provider| {
        let chars = q.chars(provider);
        chars.has(Keyword::Banding)
            || qualities(q, chars.abilities).any(|quality| {
                quality(q, provider)
                    && group
                        .iter()
                        .any(|&other| other != provider && quality(q, other))
            })
    })
}
