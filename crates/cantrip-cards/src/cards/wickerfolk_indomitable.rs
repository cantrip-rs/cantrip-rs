use cantrip_api::*;
use cantrip_mechanics::cast_from_your_graveyard;
use cantrip_mechanics::targets::filters::artifact_or_creature;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "You may cast this card from your graveyard by paying 2 life and sacrificing an artifact or creature in addition to paying
    // its other costs" (CR 601.3, 118.8, 601.2f): its own permission, with the two additional costs.
    exile_casts: &[ExileCast { additional: life(2).and(sacrifice(1, artifact_or_creature)), ..cast_from_your_graveyard::PERMISSION }],
    ..oracle::WICKERFOLK_INDOMITABLE
};
