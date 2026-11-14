use cantrip_api::select::{Choice, SelectFn, Verdict};
use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{library, select, statics, station};
use cantrip_oracle as oracle;

const SEVEN_PLUS: &str = "7+ | Flying";

/// "A land or Lander" (CR 205.2a, 205.3g): a land, or a permanent with the subtype Lander (an artifact), the subtype read through the
/// words of the asking text (CR 612.2).
fn land_or_lander(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let lander = q.words(choice.source).subtype(Subtype::Lander);
    Verdict::each(choice, |id| {
        let chars = q.chars(id);
        chars.is(CardType::Land) || chars.has_subtype(lander)
    })
}

/// "You may sacrifice a land or Lander. If you do, search your library for up to two basic land cards, put them onto the
/// battlefield tapped, then shuffle." (CR 701.21a, 701.23, 701.24): declining is the empty answer to an up-to-one choice, and
/// only a sacrifice that happened lets the search happen; fewer than two lands may be found, or none, and the library is
/// shuffled whenever it is searched.
fn sacrifice_to_search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = select::cards(select::up_to(1)).with(select::part(&(land_or_lander as SelectFn), 0, "")).called("land or Lander");
    library::may_sacrifice_to_find_to_battlefield_tapped(g, item, rule, select::basic_land_cards(select::up_to(2)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            sacrifice_to_search,
            "When this Spacecraft enters, you may sacrifice a land or Lander. If you do, search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle.",
        ),
        station::ability("Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 7+.)"),
    ],
    statics: &[
        static_ability(Layer::Type, station::charged::<7>, station::is_a_creature, SEVEN_PLUS),
        static_ability(Layer::Ability, statics::itself, statics::have_flying, SEVEN_PLUS).continues(),
        static_ability(Layer::PtSet, statics::itself, statics::base_pt::<3, 3>, SEVEN_PLUS).continues(),
    ],
    ..oracle::LARVAL_SCOUTLANDER
};
