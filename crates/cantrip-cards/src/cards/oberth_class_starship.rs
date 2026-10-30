use cantrip_api::*;
use cantrip_mechanics::{library, select, statics, station, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

const SIX_PLUS: &str = "6+ | Flying";

/// "Search your library for a basic Plains card, reveal it, put it into your hand, then shuffle" (CR 701.23b, 701.20,
/// 701.24): a basic land is searched for, the player may fail to find, and the search itself shuffles.
fn fetch_plains(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_hand(g, item.controller, item.object, select::BASIC_PLAINS_CARD, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, fetch_plains,
            "When this Spacecraft enters, search your library for a basic Plains card, reveal it, put it into your hand, then shuffle."),
        station::ability("Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 6+.)"),
    ],
    statics: &[
        static_ability(Layer::Type, station::charged::<6>, station::is_a_creature, SIX_PLUS),
        static_ability(Layer::Ability, station::charged::<6>, statics::have_flying, SIX_PLUS).continues(),
        static_ability(Layer::PtSet, station::charged::<6>, statics::base_pt::<2, 2>, SIX_PLUS).continues(),
    ],
    ..oracle::OBERTH_CLASS_STARSHIP
};
