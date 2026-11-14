use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_DIES, tokens::create_a_white_flying_spirit, "When this creature dies, create a 1/1 white Spirit creature token with flying.")],
    ..oracle::DOOMED_TRAVELER
};

pub static EXTRA: &[&CardDef] = &[&tokens::WHITE_FLYING_SPIRIT];
