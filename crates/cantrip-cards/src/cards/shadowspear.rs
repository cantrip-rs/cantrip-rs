use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{1}"), effects::opponents_permanents_lose_hexproof_and_indestructible, "{1}: Permanents your opponents control lose hexproof and indestructible until end of turn."),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[
        static_ability(Layer::PtModify, attached_object, cantrip_mechanics::statics::plus_pt::<1, 1>, "Equipped creature gets +1/+1."),
        static_ability(Layer::Ability, attached_object, cantrip_mechanics::statics::have_trample_and_lifelink, "Equipped creature has trample and lifelink."),
    ],
    ..oracle::SHADOWSPEAR
};
