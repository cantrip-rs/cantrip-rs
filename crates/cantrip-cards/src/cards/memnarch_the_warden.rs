use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ATTACKS, THIS_ENTERS};
use cantrip_mechanics::{artifacts_you_control, tokens};
use cantrip_oracle as oracle;

/// "Create two 1/1 colorless Myr artifact creature tokens" (CR 701.7): the two are made together.
fn two_myr(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::MYR, 2)?;
    Ok(())
}

/// "Draw a card for each artifact you control" (CR 121.2): the count is taken as the ability resolves, and only the
/// attacking player's artifacts count, whichever controller they belong to.
fn draw_per_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let count = artifacts_you_control(g, item.controller) as u32;
    g.draw(item.controller, count);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, two_myr, "When Memnarch enters, create two 1/1 colorless Myr artifact creature tokens."),
        triggered(THIS_ATTACKS, draw_per_artifact, "Whenever Memnarch attacks, draw a card for each artifact you control."),
    ],
    ..oracle::MEMNARCH_THE_WARDEN
};

pub static EXTRA: &[&CardDef] = &[&tokens::MYR];
