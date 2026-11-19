use cantrip_api::*;
use cantrip_mechanics::triggers::{you_attack_with_a_creature_of_subtype, ARTIFACT_SPELL_YOU_CAST};
use cantrip_mechanics::{lands, tokens};
use cantrip_oracle as oracle;

/// "Whenever one or more Dinosaurs you control attack, create that many Treasure tokens" (CR 508.1m, 603.2c, 111.10a): the
/// Dinosaurs declared together are one batch (`once_per_batch`), each its subject, so the ability reads how many
/// (`Query::event_objects`) and makes one Treasure for each, together (CR 101.4).
fn that_many_treasures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let many = g.event_objects(item).len() as u32;
    tokens::create_by_rule(g, item.controller, &tokens::TREASURE, many, false)?;
    Ok(())
}

/// "Create a 3/1 red Dinosaur creature token" (CR 111.1).
fn create_a_dinosaur(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::DINOSAUR, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ATTACKS, you_attack_with_a_creature_of_subtype::<{ lands::sub(Subtype::Dinosaur) }>),
            that_many_treasures,
            "Whenever one or more Dinosaurs you control attack, create that many Treasure tokens.",
        )
        .once_per_batch(),
        triggered(
            ARTIFACT_SPELL_YOU_CAST,
            create_a_dinosaur,
            "Whenever you cast an artifact spell, create a 3/1 red Dinosaur creature token. This ability triggers only once each turn.",
        )
        .triggers_each_turn(1),
    ],
    ..oracle::POETIC_INGENUITY
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE, &tokens::DINOSAUR];
