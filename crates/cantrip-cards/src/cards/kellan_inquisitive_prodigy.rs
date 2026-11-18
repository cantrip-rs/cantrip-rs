use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Destroy up to one target artifact. If you controlled that permanent, draw a card." Whether you controlled it is read as the
/// artifact was before it is destroyed (CR 608.2h); the card is drawn whether or not the destruction happened (an indestructible
/// artifact you control still counts). With the target gone or illegal, or none chosen, nothing happens (CR 608.2b).
fn destroy_an_artifact_and_draw_if_yours(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(artifact) = targets::object(g, item, 0) else { return Ok(()) };
    let yours = g.controller(artifact) == item.controller;
    g.destroy(artifact);
    if yours {
        effects::draw(g, item, 1)?;
    }
    Ok(())
}

/// "Investigate. You may play an additional land this turn." (CR 701.16a, 305.2): two sentences of one resolution, the Clue first.
fn investigate_and_play_an_additional_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)?;
    g.additional_land_plays(item, 1);
    Ok(())
}

/// The Adventure (CR 715): Tail the Suspect.
pub static TAIL_THE_SUSPECT: CardDef = CardDef { spell: spell(&[], investigate_and_play_an_additional_land), ..oracle::TAIL_THE_SUSPECT };

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        destroy_an_artifact_and_draw_if_yours,
        "Whenever Kellan attacks, destroy up to one target artifact. If you controlled that permanent, draw a card.",
    )
    .targets(&[targets::ARTIFACT.between(0, 1)])],
    adventure: Some(&TAIL_THE_SUSPECT),
    ..oracle::KELLAN_INQUISITIVE_PRODIGY
};

pub static EXTRA: &[&CardDef] = &[&TAIL_THE_SUSPECT, &tokens::CLUE];
