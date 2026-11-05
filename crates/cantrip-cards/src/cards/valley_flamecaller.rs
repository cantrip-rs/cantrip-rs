use cantrip_api::*;
use cantrip_oracle as oracle;

/// "If a Lizard, Mouse, Otter, or Raccoon you control would deal damage to a permanent or player": damage, combat or not, of a
/// source of one of those creature types that its controller controls (CR 614.1, 120.3), to anything. The types are words of
/// this card's text (CR 612). It adds to damage that is still going to be dealt: damage already prevented to nothing is no
/// damage (CR 615.6), so there is none to add to. Valley Flamecaller is a Lizard itself.
fn damage_from_a_lizard_mouse_otter_or_raccoon(q: &dyn Query, asker: &Asker, p: &Proposal) -> bool {
    let Proposal::Damage(d) = p else { return false };
    let words = q.words(asker.source);
    let chars = q.chars(d.source);
    d.amount > 0
        && q.controller(d.source) == asker.controller
        && [Subtype::Lizard, Subtype::Mouse, Subtype::Otter, Subtype::Raccoon]
            .into_iter()
            .any(|printed| chars.has_subtype(words.subtype(printed)))
}

fn plus_one(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.amount = d.amount.checked_add(1).expect("damage expression exceeds i64 semantic domain");
    }
}

pub static CARD: CardDef = CardDef {
    replacements: &[replacement(ReplacementKind::Damage, damage_from_a_lizard_mouse_otter_or_raccoon, plus_one, "deals that much damage plus 1 instead")],
    ..oracle::VALLEY_FLAMECALLER
};
