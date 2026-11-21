use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, create_a_token_then_attach_this, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            |g, item| create_a_token_then_attach_this(g, item, &tokens::ALLY),
            "When this Equipment enters, create a 1/1 white Ally creature token, then attach this Equipment to it.",
        ),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<1, 0>, "Equipped creature gets +1/+0.")],
    ..oracle::KYOSHI_BATTLE_FAN
};

pub static EXTRA: &[&CardDef] = &[&tokens::ALLY];
