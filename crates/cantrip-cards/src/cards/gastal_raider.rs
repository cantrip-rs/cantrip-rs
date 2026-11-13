use cantrip_api::*;
use cantrip_mechanics::hand::discard_chosen_from_revealed_hand_of_kind;
use cantrip_mechanics::max_speed::itself_at_max_speed;
use cantrip_mechanics::statics::{have_menace, plus_pt};
use cantrip_mechanics::{targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Target opponent reveals their hand. You choose an instant or sorcery card from it. That player discards that card." (CR
/// 701.9a, 701.20a): asked even when nothing in the hand is eligible, as looking at the hand is part of the ability.
fn discard_instant_or_sorcery(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    discard_chosen_from_revealed_hand_of_kind(g, item, opponent, Purpose::TakeFromHand, "instant or sorcery", |q, card| {
        let chars = q.chars(card);
        chars.is(CardType::Instant) || chars.is(CardType::Sorcery)
    })?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[triggered(
        THIS_ENTERS,
        discard_instant_or_sorcery,
        "When this creature enters, target opponent reveals their hand. You choose an instant or sorcery card from it. That player discards that card.",
    )
    .targets(&[targets::OPPONENT])],
    statics: &[
        static_ability(Layer::PtModify, itself_at_max_speed, plus_pt::<1, 1>, "Max speed \u{2014} This creature gets +1/+1."),
        static_ability(Layer::Ability, itself_at_max_speed, have_menace, "Max speed \u{2014} This creature has menace."),
    ],
    ..oracle::GASTAL_RAIDER
};
