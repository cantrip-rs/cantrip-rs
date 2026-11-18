use cantrip_api::*;
use cantrip_mechanics::statics::itself;
use cantrip_mechanics::{disguise, lands, library, select, triggers};
use cantrip_oracle as oracle;

/// "Up to two Forest cards" (CR 205.3i, 701.23b): a land card with the subtype Forest, basic or not, read through the words of the
/// card that asks (CR 612.2).
const FOREST_CARDS: select::Selection = select::Selection::of(select::up_to(2), &[select::FOREST]).called("Forest");

/// "Search your library for up to two Forest cards and reveal them. Put one of them onto the battlefield tapped and the other into
/// your hand, then shuffle." (CR 701.23, 701.20, 701.24): none, one or two are found; a lone card found goes onto the battlefield
/// tapped; of two, the player says which one does.
fn search_two_forests(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::search_two_one_onto_battlefield_tapped_other_into_hand(g, item, FOREST_CARDS)
}

/// "This creature gets +1/+1 for each Forest you control." (CR 613.4c, layer 7c): a face-up 0/0 that counts every Forest its controller
/// has, itself too if it were one (it is not); face down it is a 2/2 with no abilities (CR 708.2), so the static does not apply.
/// Disguise {4}{G} (CR 702.168): "When this creature is turned face up" - not when it enters (a creature cast face down never
/// triggers on entering, and one cast face up has no such event).
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        itself,
        lands::plus_per_subtype_you_control::<{ lands::sub(Subtype::Forest) }, false, 1, 1>,
        "This creature gets +1/+1 for each Forest you control.",
    )],
    abilities: &[triggered(
        triggers::THIS_TURNED_FACE_UP,
        search_two_forests,
        "When this creature is turned face up, search your library for up to two Forest cards and reveal them. Put one of them onto the battlefield tapped and the other into your hand, then shuffle.",
    )],
    ways: &[disguise::way(Cost::mana(mana("{4}{G}")), "Disguise {4}{G}")],
    ..oracle::FLOURISHING_BLOOM_KIN
};
