use cantrip_api::*;
use cantrip_mechanics::{Animation, animate, bargain, lands, targets};
use cantrip_oracle as oracle;

/// "If this spell was bargained, target land you control becomes a 3/3 Elemental creature with haste that's still a land.
/// Search your library for a basic land card, reveal it, put it into your hand, then shuffle."
///
/// The land is a target only if the spell was bargained (`TargetSpec::if_paid`, CR 601.2b-c), so unbargained there is none
/// and only the search happens. The change has no duration (CR 611.2a): it lasts while the land stays on the battlefield. The
/// text names no colors, so they are not changed, and the land keeps every type and subtype it had, creature types included. If
/// the one target is no longer legal as the spell resolves, it does not resolve and the search does not happen (CR 608.2b).
fn becomes_a_creature_then_search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(land) = targets::object(g, item, 0) {
        let animation = Animation {
            power: 3,
            toughness: 3,
            colors: None,
            types: TypeSet::NONE,
            subtypes: &[g.words(item.object).subtype(Subtype::Elemental)],
            all_creature_types: false,
            replaces_creature_types: false,
            keywords: Keywords::of(&[Keyword::Haste]),
            ability: None,
        };
        animate(g, item.object, land, &animation, Duration::Indefinite);
    }
    lands::search_basic_land_to_hand(g, item.controller, item.object)
}

pub static CARD: CardDef = CardDef {
    optional_cost: Some(bargain::OPTIONAL_COST),
    labels: bargain::LABELS,
    spell: spell(&[targets::LAND_YOU_CONTROL.between(0, 0).if_paid(1, 1)], becomes_a_creature_then_search),
    ..oracle::BRAVE_THE_WILDS
};
