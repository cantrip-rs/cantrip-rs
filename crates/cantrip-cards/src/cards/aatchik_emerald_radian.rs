use cantrip_api::*;
use cantrip_mechanics::{effects, lands, tokens, triggers};
use cantrip_oracle as oracle;

/// "Create a 1/1 green Insect creature token for each artifact and/or creature card in your graveyard" (CR 111.1, 404.1, 700.1):
/// the count is of cards (never tokens) that are an artifact, a creature or both, counted once each, as the ability resolves.
fn create_an_insect_for_each_artifact_or_creature_card_in_your_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let count = g
        .state()
        .zone(Zone::graveyard(item.controller))
        .iter()
        .filter(|&&card| g.chars(card).is(CardType::Artifact) || g.chars(card).is(CardType::Creature))
        .count();
    tokens::create(g, item.controller, &tokens::INSECT, count as u32)?;
    Ok(())
}

/// "Put a +1/+1 counter on Aatchik. Each opponent loses 1 life."
fn counter_and_drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counter_on_this(g, item)?;
    effects::each_opponent_loses_1_life(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            create_an_insect_for_each_artifact_or_creature_card_in_your_graveyard,
            "When Aatchik enters, create a 1/1 green Insect creature token for each artifact and/or creature card in your graveyard.",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::another_of_subtype_you_control_dies::<{ lands::sub(Subtype::Insect) }>),
            counter_and_drain,
            "Whenever another Insect you control dies, put a +1/+1 counter on Aatchik. Each opponent loses 1 life.",
        ),
    ],
    ..oracle::AATCHIK_EMERALD_RADIAN
};

pub static EXTRA: &[&CardDef] = &[&tokens::INSECT];
