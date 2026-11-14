use cantrip_api::*;
use cantrip_mechanics::{choose, lands, library, untapped_subtype_you_control};
use cantrip_oracle as oracle;

/// "Exile cards from the top of your library until you exile a nonland card. You may cast that card without paying its mana
/// cost." (CR 701.13a, 607.2a, 608.2g, 118.9): only the nonland card this resolution stopped on is offered, not the lands around
/// it nor a card an earlier resolution exiled.
fn exile_until_nonland_then_cast_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let by = g.linked(item);
    let Some(card) = library::exile_until_any_nonland(g, item.controller, Some(by)) else { return Ok(()) };
    choose::cast_up_to_free(g, item.controller, item.object, &[card], 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        tap_untapped(3, untapped_subtype_you_control::<{ lands::sub(Subtype::Faerie) }>),
        exile_until_nonland_then_cast_it,
        "Tap three untapped Faeries you control: Exile cards from the top of your library until you exile a nonland card. You may cast that card without paying its mana cost.",
    )],
    ..oracle::Q_YOUR_ILLUSTRIOUS_HOST
};
