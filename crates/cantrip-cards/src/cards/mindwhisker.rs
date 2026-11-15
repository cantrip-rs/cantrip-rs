use cantrip_api::*;
use cantrip_mechanics::statics::{plus_pt, source_has_threshold, your_opponents_creatures};
use cantrip_mechanics::triggers::YOUR_UPKEEP;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Threshold \u{2014} As long as there are seven or more cards in your graveyard, creatures your opponents control get -1/-0"
/// (CR 207.2c, 604.1, 611.3a): the creatures are those of its controller's opponents, and the count of its controller's own
/// graveyard (never an opponent's), each asked again whenever characteristics are computed.
fn opponents_creatures_while_you_have_threshold(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    your_opponents_creatures(q, source, affected, so_far) && source_has_threshold(q, source)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(YOUR_UPKEEP, |g, item| effects::surveil(g, item, 1), "At the beginning of your upkeep, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)")],
    statics: &[static_ability(
        Layer::PtModify,
        opponents_creatures_while_you_have_threshold,
        plus_pt::<-1, 0>,
        "Threshold \u{2014} As long as there are seven or more cards in your graveyard, creatures your opponents control get -1/-0.",
    )],
    ..oracle::MINDWHISKER
};
