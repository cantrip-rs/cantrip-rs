use cantrip_api::*;
use cantrip_mechanics::{blight, effects, targets};
use cantrip_oracle as oracle;

/// "Return target creature card from your graveyard to the battlefield. Then if it isn't your main phase, blight 2" (CR 601.2c,
/// 701.68a, 505.1): the main phase is judged as the spell resolves, a precombat or postcombat main phase of its controller's turn.
/// The blight happens whether or not the card came back (a Grafdigger's Cage keeps it in the graveyard), and the creature that did
/// is among those that can take the counters.
fn return_then_blight_off_your_main_phase(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_first_target_to_battlefield(g, item)?;
    let turn = g.state().turn;
    if !(turn.active == item.controller && turn.step().is_main()) {
        blight::in_effect(g, item.controller, item.object, 2)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD], return_then_blight_off_your_main_phase),
    ..oracle::DOSE_OF_DAWNGLOW
};
