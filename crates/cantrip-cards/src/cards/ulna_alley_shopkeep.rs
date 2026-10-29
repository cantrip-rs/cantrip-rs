use cantrip_api::*;
use cantrip_mechanics::statics::{itself, plus_pt};
use cantrip_oracle as oracle;

/// "As long as you gained life this turn" (CR 611.3a): itself, while its controller has gained life this turn. The layers ask it
/// again whenever characteristics are computed, so it turns on with the gain and off as the turn ends.
fn itself_while_you_gained_life(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    itself(view, source, affected, so_far) && view.history().life_gained(view.state().obj(source).controller) > 0
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        itself_while_you_gained_life,
        plus_pt::<2, 0>,
        "Infusion \u{2014} This creature gets +2/+0 as long as you gained life this turn.",
    )],
    ..oracle::ULNA_ALLEY_SHOPKEEP
};
