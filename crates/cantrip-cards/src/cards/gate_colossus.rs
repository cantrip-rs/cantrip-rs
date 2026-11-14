use cantrip_api::*;
use cantrip_mechanics::{affinity, effects, lands, triggers};
use cantrip_oracle as oracle;

const PUT_ON_TOP: Question = Question::new("PutOnTopOfLibrary", "put it from your graveyard on top of your library?", "", "Put it on top", "Leave it");

/// "You may put this card from your graveyard on top of your library" (CR 113.6k, 400.7): asked only while it is still the
/// card that triggered, and put on top of its owner's library on a yes.
fn may_put_on_top_of_your_library(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::ask_may_this_card(g, item, Purpose::Ask(&PUT_ON_TOP))? {
        g.put_into_library(&[item.object], LibraryPlace::Top);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // "Affinity for Gates" (CR 702.41a); "This creature can't be blocked by creatures with power 2 or less" (CR 509.1b).
    own_cost_changes: &[affinity::for_land_type::<{ lands::sub(Subtype::Gate) }>],
    keywords: Keywords::of(&[Keyword::CantBeBlockedByPowerTwoOrLess]),
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::of_subtype_you_control_enters::<{ lands::sub(Subtype::Gate) }>),
        may_put_on_top_of_your_library,
        "Whenever a Gate you control enters, you may put this card from your graveyard on top of your library.",
    )
    // It puts this card from the graveyard, so it functions only there (CR 113.6m).
    .functions_in(IN_A_GRAVEYARD)],
    ..oracle::GATE_COLOSSUS
};
