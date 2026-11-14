use cantrip_api::*;
use cantrip_mechanics::attach::equip;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::LANDFALL;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(LANDFALL, effects::attached_creature_gets::<2, 2>, "Landfall \u{2014} Whenever a land you control enters, equipped creature gets +2/+2 until end of turn."),
        equip(mana("{1}"), "Equip {1}"),
    ],
    ..oracle::ADVENTURING_GEAR
};
