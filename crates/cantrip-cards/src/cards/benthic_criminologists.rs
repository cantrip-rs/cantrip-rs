use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS_OR_ATTACKS;
use cantrip_mechanics::{effects, select};
use cantrip_oracle as oracle;

/// "You may sacrifice an artifact. If you do, draw a card." (CR 701.21a, 608.2c-d): the artifact is chosen as this resolves, not as a cost
/// (this creature is one if it is an artifact; it isn't), and the card is drawn only if one was sacrificed.
fn may_sacrifice_an_artifact_to_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::may_sacrifice(g, item.controller, item.object, select::artifact_cards(select::up_to(1)))?.is_empty() {
        g.draw(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS_OR_ATTACKS,
        may_sacrifice_an_artifact_to_draw,
        "Whenever this creature enters or attacks, you may sacrifice an artifact. If you do, draw a card.",
    )],
    ..oracle::BENTHIC_CRIMINOLOGISTS
};
