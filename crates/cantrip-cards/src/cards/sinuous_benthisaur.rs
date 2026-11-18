use cantrip_api::*;
use cantrip_mechanics::{permanents_with_subtype_you_control, subtype_cards_in_graveyard};
use cantrip_mechanics::library::{look_at_top, Rest};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this creature enters, look at the top X cards of your library, where X is the number of Caves you control plus the number of Cave
/// cards in your graveyard. Put two of those cards into your hand and the rest on the bottom of your library in a random order." (CR 401.4,
/// 609.3): X is counted as the ability resolves; "Cave" is a land type word of its text (CR 205.3i, 612.2), which a Cave on the battlefield
/// has as the layers make it and a card in the graveyard has as printed. A library with fewer than X cards shows what it has, and only as
/// many cards as there are can go to the hand.
fn look_at_top_cave_cards(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let cave = g.words(item.object).subtype(Subtype::Cave);
    let x = permanents_with_subtype_you_control(g, you, cave) + subtype_cards_in_graveyard(g, you, cave);
    look_at_top(g, you, item.object, x, (2, 2), false, |_| true, Rest::BottomRandom)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        look_at_top_cave_cards,
        "When this creature enters, look at the top X cards of your library, where X is the number of Caves you control plus the number of Cave cards in your graveyard. Put two of those cards into your hand and the rest on the bottom of your library in a random order.",
    )],
    ..oracle::SINUOUS_BENTHISAUR
};
