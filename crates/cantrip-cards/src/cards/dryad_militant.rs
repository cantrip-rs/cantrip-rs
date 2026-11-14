use cantrip_api::*;
use cantrip_mechanics::replacements;
use cantrip_oracle as oracle;

/// "If an instant or sorcery card would be put into a graveyard from anywhere, exile it instead" (CR 614.1a, 400.3): the card as
/// it is where it is now - a spell on the stack, an instant or sorcery in a hand or a library being discarded or milled. A copy
/// of a spell is not a card (CR 108.2, 707.10), so it is not replaced.
fn instant_or_sorcery_card_to_a_graveyard(q: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.to.is(ZoneKind::Graveyard) && z.is_card && {
        let c = q.chars(z.object);
        c.is(CardType::Instant) || c.is(CardType::Sorcery)
    })
}

pub static CARD: CardDef = CardDef {
    replacements: &[replacement(
        ReplacementKind::ZoneChange,
        instant_or_sorcery_card_to_a_graveyard,
        replacements::exile_instead_apply,
        "exiles an instant or sorcery card a graveyard would receive",
    )],
    ..oracle::DRYAD_MILITANT
};
