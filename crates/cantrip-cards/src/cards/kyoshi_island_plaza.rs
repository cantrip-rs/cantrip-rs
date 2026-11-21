use cantrip_api::*;
use cantrip_mechanics::{lands, library, permanents_with_subtype_you_control, select, triggers};
use cantrip_oracle as oracle;

/// "Search your library for up to X basic land cards, where X is the number of Shrines you control. Put those cards onto the
/// battlefield tapped, then shuffle." (CR 701.23, 608.2c): X is counted as the ability resolves, the Shrines its controller
/// controls then, this one included if it is still on the battlefield, through the words of the Plaza's text (CR 612); the
/// library is searched and shuffled even when X is 0.
fn search_for_a_basic_land_per_shrine(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let shrine = g.words(item.object).subtype(Subtype::Shrine);
    let x = permanents_with_subtype_you_control(g, item.controller, shrine) as u32;
    library::find_to_battlefield(g, item.controller, item.object, select::basic_land_cards(select::up_to(x)), true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            search_for_a_basic_land_per_shrine,
            "When Kyoshi Island Plaza enters, search your library for up to X basic land cards, where X is the number of Shrines you control. Put those cards onto the battlefield tapped, then shuffle.",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Shrine) }>),
            |g, item| lands::search_basic_land_to_battlefield(g, item.controller, item.object, true),
            "Whenever another Shrine you control enters, search your library for a basic land card, put it onto the battlefield tapped, then shuffle.",
        ),
    ],
    ..oracle::KYOSHI_ISLAND_PLAZA
};
