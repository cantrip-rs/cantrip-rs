use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Target instant or sorcery card from a graveyard" (CR 115.1, 404.1): any player's graveyard.
fn instant_or_sorcery_card_in_a_graveyard(q: &dyn Query, _: &Asker, t: Target) -> bool {
    targets::card_in_graveyard(q, t, &[CardType::Instant, CardType::Sorcery])
}

/// "You may cast target instant or sorcery card from a graveyard, and mana of any type can be spent to cast that spell. If that
/// spell would be put into a graveyard, exile it instead" (CR 608.2g, 609.4b, 614.1a): cast as the ability resolves, as part of
/// it (the ruling), so it cannot wait for later in the turn; the replacement follows the spell and applies to its move to a
/// graveyard.
fn cast_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_cast_first_target_with_mana_of_any_type(g, item, ExileInstead::Spell)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        cast_it,
        "Blue Magic — When Quistis Trepe enters, you may cast target instant or sorcery card from a graveyard, and mana of any type can be spent to cast that spell. If that spell would be put into a graveyard, exile it instead.",
    )
    .targets(&[target(instant_or_sorcery_card_in_a_graveyard)])],
    ..oracle::QUISTIS_TREPE
};
