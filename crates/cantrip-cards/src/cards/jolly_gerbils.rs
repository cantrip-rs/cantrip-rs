use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever you give a gift" (CR 702.174c, 603.2): once for each time the source's controller gives a gift, the moment the
/// gift is given - as the spell resolves (CR 702.174j) or as the permanent's gift triggered ability resolves (CR 702.174b).
/// A gift an opponent gives, or one that is promised but never given (a countered spell), does not trigger it.
fn you_give_a_gift(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::GaveGift { player, .. } if *player == me.controller => Fired::about(Target::Player(*player)),
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when_asked(EventMask::GAVE_GIFT, you_give_a_gift), |g, item| effects::draw(g, item, 1), "Whenever you give a gift, draw a card.")],
    ..oracle::JOLLY_GERBILS
};
