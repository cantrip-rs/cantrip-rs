//! Assimilate a card as the printed reminder defines it: put the creature card onto the battlefield under the ability
//! controller's control with a +1/+1 counter; it is a Borg artifact creature and loses its other creature types. Under
//! CR 205.1b, "artifact creature" retains prior card types and subtypes other than creature types; the Borg subtype replaces
//! its prior creature subtypes. Assimilation Assessor, Borg Cube, Borg Queen, Perfection Manifest, Death Is Irrelevant,
//! Assimilate the Wounded, Gift of Perfection, The Best of Both Worlds, and The Borg Queen print this action.

use cantrip_api::*;

static AS_BORG_ARTIFACT_CREATURE: EntersAs = EntersAs {
    adds_types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    loses_subtypes_of: &[CardType::Creature],
    subtypes: &[Subtype::Borg],
    ..EntersAs::NONE
};

/// Assimilate `card` as it enters: add artifact and creature types, replace its creature types with Borg, and add the
/// +1/+1 counter before entry replacements and ETB triggers (CR 205.1b, 400.7, 614.1c, 614.12). `None` means the effect
/// could not put it onto the battlefield.
pub fn assimilate(g: &mut dyn Game, item: &StackItem, card: ObjectId) -> Result<Option<ObjectId>, Suspend> {
    let entered = g.put_all_onto_battlefield_becoming(&[(card, item.controller)], item, &AS_BORG_ARTIFACT_CREATURE, &[(CounterKind::PlusOne, 1)])?;
    Ok(entered.first().copied())
}
