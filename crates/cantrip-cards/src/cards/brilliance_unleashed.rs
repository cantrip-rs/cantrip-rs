use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// "Brilliance Unleashed deals 5 damage to target creature."
fn damage_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 5);
    Ok(())
}

/// "Target artifact card in your graveyard."
fn artifact_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Artifact])
}

/// "It's a 3/3 Robot artifact creature with flying": what a card that was not an artifact creature enters as (CR 614.12, 205.1a,
/// 613.1d, 613.4b), from the moment it enters, so what watches creatures enter sees it.
static AS_A_ROBOT: EntersAs = EntersAs {
    types: Some(TypeSet::of(&[CardType::Artifact, CardType::Creature])),
    subtypes: &[Subtype::Robot],
    power_toughness: Some((3, 3)),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..EntersAs::NONE
};

/// "Choose target artifact card in your graveyard. Return it to the battlefield if it's an artifact creature card. Otherwise,
/// return it to the battlefield and it's a 3/3 Robot artifact creature with flying." With the target gone or no longer such a
/// card nothing is returned (CR 608.2b).
fn return_the_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    if g.chars(card).is(CardType::Creature) {
        g.put_onto_battlefield(card, item.controller, false)?;
    } else {
        g.put_onto_battlefield_as(card, item.controller, item, &AS_A_ROBOT)?;
    }
    Ok(())
}

/// "Choose one or both" (CR 700.2): the modes are carried out in the order printed (CR 608.2c).
static MODAL: ModalDef = modes(&[
    mode(damage_creature, "Brilliance Unleashed deals 5 damage to target creature.").targets(&[targets::CREATURE]),
    mode(
        return_the_artifact,
        "Choose target artifact card in your graveyard. Return it to the battlefield if it's an artifact creature card. Otherwise, return it to the battlefield and it's a 3/3 Robot artifact creature with flying.",
    )
    .targets(&[target(artifact_card_in_your_graveyard)]),
])
.choose(1, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::BRILLIANCE_UNLEASHED };
