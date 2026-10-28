use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target instant and/or sorcery cards from your graveyard" (CR 115.1): each an instant card or a sorcery card.
fn instant_or_sorcery_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Instant, CardType::Sorcery])
}

/// "Return up to X target instant and/or sorcery cards from your graveyard to your hand. Exile Divergent Equation." The cards
/// still legal return together (CR 608.2b); the spell itself, on the stack, is not in the graveyard to be chosen. Exiling
/// itself is its last instruction (CR 608.2c), so it goes to exile rather than to its owner's graveyard, even when no target
/// was chosen or none is still legal.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let returning = targets::legal_objects(g, item);
    g.return_to_hand(&returning);
    g.end_sentence();
    g.exile(item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(instant_or_sorcery_card_in_your_graveyard).between(0, TargetSpec::COUNTED)], effect),
    ..oracle::DIVERGENT_EQUATION
};
