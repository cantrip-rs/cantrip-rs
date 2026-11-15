use cantrip_api::*;
use cantrip_mechanics::{effects, lands, triggers};
use cantrip_oracle as oracle;

/// "When this artifact enters and when you sacrifice it, you may search your library for a basic land card, put it onto the battlefield
/// tapped, then shuffle." (CR 603.6a, 701.21, 701.23): `.on_sacrifice()` is the second trigger event, as on Carrot Cake.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS_OR_YOU_SACRIFICE_IT,
            |g, item| lands::may_fetch_basic(g, item.controller, item.object, true),
            "When this artifact enters and when you sacrifice it, you may search your library for a basic land card, put it onto the battlefield tapped, then shuffle.",
        )
        .on_sacrifice(),
        ability(cost("{2}, {T}").and(SACRIFICE_THIS), effects::gain_three_life, "{2}, {T}, Sacrifice this artifact: You gain 3 life."),
    ],
    ..oracle::HEAPED_HARVEST
};
