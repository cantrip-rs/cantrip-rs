use cantrip_api::*;
use cantrip_mechanics::recruit;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, recruit::effect, "When this creature enters, recruit. (Draw a card, then discard a card. If you discarded a nonland card, create a 1/1 white Human Soldier creature token.)")],
    ..oracle::LONG_LAKE_NUISANCE
};

pub static EXTRA: &[&CardDef] = &[&cantrip_mechanics::tokens::HUMAN_SOLDIER];
