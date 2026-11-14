//! Limits on the one simultaneous untap of the active player (CR 502.3).
use cantrip_api::*;
use crate::{Cx, R};

/// Only live source texts are visited, including copy/component links. An empty
/// rule list never asks characteristics or allocates on behalf of this feature.
fn rules<'a>(q: &'a dyn Query, cards: Option<&'a CardTable>, player: PlayerId) -> impl Iterator<Item = (Asker, &'static UntapLimit)> + 'a {
    q.state().battlefield().iter().flat_map(move |&source| {
        let obj = q.state().obj(source);
        // Validation has only Query; the ordinary untap walk uses its table and
        // avoids computing characteristics in games with no limit sources.
        let current = cards.map_or_else(|| q.chars(source).card, |cards| cards.current(obj));
        let components = match obj.copy {
            Some(ref c) if c.merged != 0 && obj.marks.face_down_as.is_none() => q.state().merged_sources(c).len(),
            _ => 0,
        };
        CardTable::texts_of(q.state(), obj, current).enumerate().flat_map(move |(i, card)| {
            let by = (i > 0 && i <= components).then(|| Linked { object: source, text: q.linked_card(card), acquired: q.state().copy_acquired(source) });
            let asker = Asker { source, controller: obj.controller, by, spell: false, instance: None };
            q.card(card).untap_limits.iter().filter(move |r| !q.lost_all_abilities(source) && (r.applies)(q, &asker, player)).map(move |r| (asker, r))
        })
    })
}

/// All limits judge one set. Sequential per-type choices would incorrectly lose
/// alternatives when, for example, one creature is also a land (Smoke's ruling).
/// AsManyAsCan demands an inclusion-maximal set, not a largest-cardinality set:
/// choosing that animated land may preclude both another land and another creature.
fn allowed(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let mut result = Verdict { complete: true, addable: Bits::new(choice.candidates.len(), true) };
    for (asker, rule) in rules(q, None, choice.chooser) {
        let matches = |id| (rule.permanents)(q, &asker, id);
        let n = choice.selected.iter().filter(|&&id| matches(id)).count();
        result.complete &= n as u64 <= u64::from(rule.amount);
        if n as u64 >= u64::from(rule.amount) {
            for (i, &id) in choice.candidates.iter().enumerate() {
                if matches(id) { result.addable.set(i, false); }
            }
        }
    }
    result
}
const LIMITS: &[Part] = &[cantrip_api::select::part(&(allowed as SelectFn), 0, "within each active untap limit")];

pub(crate) fn restrict(cx: &mut Cx, player: PlayerId, untapping: &mut Vec<ObjectId>) -> R<()> {
    let limits: Vec<_> = rules(cx, Some(cx.cards()), player).collect();
    if limits.is_empty() { return Ok(()) }
    // Impossible untaps cannot consume a quota; a stun replacement, by contrast,
    // still replaces one of the chosen untaps and therefore consumes its slot.
    untapping.retain(|&id| !crate::static_rules::cant_become_untapped(cx, cx.cards(), id)
        && !limits.iter().any(|(asker, rule)| rule.amount == 0 && (rule.permanents)(cx, asker, id)));
    let Some((source, _)) = limits.iter().find(|(asker, rule)| {
        untapping.iter().filter(|&&id| (rule.permanents)(cx, asker, id)).count() as u64 > u64::from(rule.amount)
    }) else { return Ok(()) };
    let source = source.source;
    // Permanents no limit concerns still untap without a question.
    let candidates: Vec<_> = untapping.iter().copied().filter(|&id| limits.iter().any(|(asker, rule)| (rule.permanents)(cx, asker, id))).collect();
    let count = u32::try_from(candidates.len()).unwrap_or(u32::MAX);
    let rule = Selection::of(Count::AsManyAsCan(count), LIMITS).called("permanent to untap");
    let request = RequestKind::ChooseCards { source, purpose: Purpose::Untap, cards: candidates.clone(), rule: Box::new(rule) };
    let chosen = match cx.ask_rule(player, request)? { Some(Answer::Cards(ids)) => ids, _ => Vec::new() };
    untapping.retain(|id| !candidates.contains(id) || chosen.contains(id));
    Ok(())
}
