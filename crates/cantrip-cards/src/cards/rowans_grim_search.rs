use cantrip_api::*;
use cantrip_mechanics::{bargain, effects, library};
use cantrip_oracle as oracle;

/// "If this spell was bargained, look at the top four cards of your library, then put up to two of them back on top of your library in
/// any order and the rest into your graveyard. You draw two cards and you lose 2 life." (CR 702.166b, 701.20, 401.4, 404.3, 121.1,
/// 119.3): the look is its own instruction, done before the draw whatever it does, so the cards left on top are the ones drawn.
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if bargain::bargained(item) {
        library::look_at_top_put_up_to_back_rest_to_graveyard::<4, 2>(g, item)?;
        g.end_sentence();
    }
    effects::draw_two_cards_and_lose_2_life(g, item)
}

pub static CARD: CardDef = CardDef {
    optional_cost: Some(bargain::OPTIONAL_COST),
    labels: bargain::LABELS,
    spell: spell(&[], search),
    ..oracle::ROWANS_GRIM_SEARCH
};
