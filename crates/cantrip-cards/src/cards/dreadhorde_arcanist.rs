use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Target instant or sorcery card with mana value less than or equal to this creature's power from your graveyard"
/// (CR 115.1, 202.3): judged as the ability goes on the stack, after the attack, and again as it resolves (CR 608.2b),
/// against Arcanist's power then, or as it last was on the battlefield if it has left (CR 113.7a, the rulings).
/// A power below 0 allows no card: no mana value is less than 0 (CR 202.3).
fn instant_or_sorcery_up_to_its_power(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    u32::try_from(targets::power_or_last_known(q, asker.source))
        .is_ok_and(|power| targets::card_in_your_graveyard_with_mana_value_at_most(q, asker, t, &[CardType::Instant, CardType::Sorcery], power))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ATTACKS, effects::may_cast_first_target_free_exiling_it, "Whenever this creature attacks, you may cast target instant or sorcery card with mana value less than or equal to this creature's power from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead.")
        .targets(&[target(instant_or_sorcery_up_to_its_power)])],
    ..oracle::DREADHORDE_ARCANIST
};
