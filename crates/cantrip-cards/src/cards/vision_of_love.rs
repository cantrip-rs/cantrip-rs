use cantrip_api::*;
use cantrip_mechanics::effects::may_sacrifice_or_discard_to_draw;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], |g, item| may_sacrifice_or_discard_to_draw(g, item, CardType::Artifact, 2)),
    labels: &["Sacrifice an artifact", "Discard a card", "Neither"],
    ..oracle::VISION_OF_LOVE
};
