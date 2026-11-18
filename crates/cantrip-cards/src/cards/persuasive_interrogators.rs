use cantrip_api::*;
use cantrip_mechanics::{lands, targets, tokens, triggers};
use cantrip_oracle as oracle;

fn investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)
}

/// "target opponent gets two poison counters" (CR 122.1f, 104.3d).
fn target_opponent_gets_two_poison_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(opponent) = targets::player(g, item, 0) {
        g.give_poison(opponent, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, investigate, "When this creature enters, investigate."),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::you_sacrifice_a_subtype::<{ lands::sub(Subtype::Clue) }>),
            target_opponent_gets_two_poison_counters, "Whenever you sacrifice a Clue, target opponent gets two poison counters.")
            .targets(&[targets::OPPONENT])
            .on_sacrifice(),
    ],
    ..oracle::PERSUASIVE_INTERROGATORS
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
