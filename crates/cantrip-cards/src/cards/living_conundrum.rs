use cantrip_api::*;
use cantrip_mechanics::replacements::{instead_of_drawing, you_would_draw_from_nothing};
use cantrip_mechanics::statics::{base_pt, have_flying_and_vigilance};
use cantrip_oracle as oracle;

/// "If you would draw a card while your library has no cards in it, skip that draw instead" (CR 614.1a, 121.6): the draw is replaced by
/// nothing, so it is not made and the player, drawing none, does not lose for it (CR 704.5b).
const SKIP_DRAW_FROM_AN_EMPTY_LIBRARY: ReplacementDef =
    replacement(ReplacementKind::Draw, you_would_draw_from_nothing, instead_of_drawing, "skips that draw instead of drawing from an empty library");

/// "As long as there are no cards in your library, this creature ..." (CR 604.1, 611.3a) as the statics' `affects`: only this creature,
/// and only while its controller's library is empty, asked again whenever characteristics are computed.
fn itself_while_your_library_is_empty(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    let state = q.state();
    source == affected && state.zone(Zone::library(state.obj(source).controller)).is_empty()
}

/// "Hexproof" is the core keyword from the content JSON. "If you would draw a card while your library has no cards in it, skip that draw
/// instead" is `SKIP_DRAW_FROM_AN_EMPTY_LIBRARY`; "as long as there are no cards in your library, this creature has base
/// power and toughness 10/10 and has flying and vigilance" (CR 604.1, 613.4b, 613.1f) is two continuous effects on the one condition, in
/// the layer that sets base power and toughness and in the ability layer, asked again whenever characteristics are computed. The condition
/// is its controller's own library (the opponent's is another zone).
pub static CARD: CardDef = CardDef {
    replacements: &[SKIP_DRAW_FROM_AN_EMPTY_LIBRARY],
    statics: &[
        static_ability(
            Layer::PtSet,
            itself_while_your_library_is_empty,
            base_pt::<10, 10>,
            "As long as there are no cards in your library, this creature has base power and toughness 10/10.",
        ),
        static_ability(
            Layer::Ability,
            itself_while_your_library_is_empty,
            have_flying_and_vigilance,
            "As long as there are no cards in your library, this creature has flying and vigilance.",
        ),
    ],
    ..oracle::LIVING_CONUNDRUM
};
