use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{crew, effects, lands};
use cantrip_oracle as oracle;

/// "When this Vehicle enters, search your library for a basic land card, reveal it, put it into your hand, then shuffle"
/// (CR 701.23): not optional, though the player may find nothing, and the library is shuffled either way.
fn fetch_a_basic(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    lands::search_basic_land_to_hand(g, item.controller, item.object)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            fetch_a_basic,
            "When this Vehicle enters, search your library for a basic land card, reveal it, put it into your hand, then shuffle.",
        ),
        ability(
            cost("{W}{U}{B}{R}{G}"),
            effects::this_vehicle_becomes_artifact_creature_with_plus_one_counters::<2>,
            "Exhaust \u{2014} {W}{U}{B}{R}{G}: This Vehicle becomes an artifact creature. Put two +1/+1 counters on it. (Activate each exhaust ability only once.)",
        )
        .exhaust(),
        crew::crew(5, "Crew 5"),
    ],
    ..oracle::MARSHALS_PATHCRUISER
};
