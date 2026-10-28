use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::recruit;
use cantrip_mechanics::triggers::YOU_ATTACK;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(YOU_ATTACK, recruit::effect, "Whenever you attack, recruit. (Draw a card, then discard a card. If you discarded a nonland card, create a 1/1 white Human Soldier creature token.)"),
        crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::GREAT_GILDED_BOAT
};

pub static EXTRA: &[&CardDef] = &[&cantrip_mechanics::tokens::HUMAN_SOLDIER];
