use cantrip_api::*;
use cantrip_mechanics::{lands, library, select, statics};
use cantrip_oracle as oracle;

/// "Search your library for a Plan card, reveal it, put it into your hand, then shuffle." (CR 701.23): the whole library is
/// shown; the player may find nothing, and the library is shuffled either way. "A Plan card" is a card with the Plan subtype,
/// read through the words of the text asking (CR 205.3, 612).
fn search_for_a_plan(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = Selection::new(select::up_to(1)).with(select::subtype_card(Subtype::Plan)).called("Plan");
    library::find_to_hand(g, item.controller, item.object, rule, true)?;
    Ok(())
}

/// "Other Villains you control get +2/+1." (CR 613.4c, layer 7c), and an ability that works from the hand: "{1}{B}, Discard this
/// card" (CR 113.6, 602.2), the discard being a cost paid as it is activated.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}{B}").and(DISCARD_THIS), search_for_a_plan, "{1}{B}, Discard this card: Search your library for a Plan card, reveal it, put it into your hand, then shuffle.")
        .from(ZoneKind::Hand)],
    statics: &[static_ability(
        Layer::PtModify,
        statics::other_of_subtype_you_control::<{ lands::sub(Subtype::Villain) }>,
        statics::plus_pt::<2, 1>,
        "Other Villains you control get +2/+1.",
    )],
    ..oracle::THE_MASTERS_OF_EVIL
};
