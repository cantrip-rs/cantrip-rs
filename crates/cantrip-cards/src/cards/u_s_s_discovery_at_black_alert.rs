use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS_OR_ATTACKS;
use cantrip_mechanics::{discover, statics, station};
use cantrip_oracle as oracle;

const SIX_PLUS: &str = "6+ | Flying (5/5)";

/// "Whenever U.S.S. Discovery enters or attacks, discover 5" (CR 701.57a, 603.6a, 508.1m): one ability with two trigger
/// events, not two abilities.
fn discover_five(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    discover::discover(g, item.controller, 5)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS_OR_ATTACKS,
            discover_five,
            "Whenever U.S.S. Discovery enters or attacks, discover 5.",
        ),
        station::ability("Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 6+.)"),
    ],
    statics: &[
        static_ability(Layer::Type, station::charged::<6>, station::is_a_creature, SIX_PLUS),
        static_ability(Layer::Ability, station::charged::<6>, statics::have_flying, SIX_PLUS).continues(),
        static_ability(Layer::PtSet, station::charged::<6>, statics::base_pt::<5, 5>, SIX_PLUS).continues(),
    ],
    ..oracle::U_S_S_DISCOVERY_AT_BLACK_ALERT
};
