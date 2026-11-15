//! Restricted Office // Lecture Hall, a Room (CR 709.5): each door is only what its text does; the Room has the abilities
//! of its unlocked doors (`layers::doors`, `CardTable::texts`).

use cantrip_api::*;
use cantrip_mechanics::{statics, triggers};
use cantrip_oracle as oracle;

/// Restricted Office: "When you unlock this door, destroy all creatures with power 3 or greater." Each creature's power is
/// read as the ability resolves (CR 608.2h), and they are destroyed at once (CR 701.8a, 101.4); a creature with no power is
/// not among them.
fn destroy_power_three_or_greater(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    let doomed: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Creature) && g.chars(id).power.is_some_and(|p| p >= 3))
        .collect();
    g.destroy_all(&doomed);
    Ok(())
}

pub static RESTRICTED_OFFICE: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_UNLOCK_THIS_LEFT_DOOR,
        destroy_power_three_or_greater,
        "When you unlock this door, destroy all creatures with power 3 or greater.",
    )],
    ..oracle::RESTRICTED_OFFICE
};

/// Lecture Hall: "Other permanents you control have hexproof" (CR 702.11, layer 6), while this door is unlocked; the Room
/// itself is not among them.
pub static LECTURE_HALL: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        statics::other_permanents_you_control,
        statics::have_hexproof,
        "Other permanents you control have hexproof.",
    )],
    ..oracle::LECTURE_HALL
};

pub static CARD: CardDef = CardDef { split: Some((&RESTRICTED_OFFICE, &LECTURE_HALL)), ..oracle::RESTRICTED_OFFICE_LECTURE_HALL };

pub static EXTRA: &[&CardDef] = &[&RESTRICTED_OFFICE, &LECTURE_HALL];
