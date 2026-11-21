use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, create_a_token_then_attach_this, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            |g, item| create_a_token_then_attach_this(g, item, &tokens::DWARF),
            "When this Equipment enters, create a 2/2 red Dwarf creature token, then attach this Equipment to it.",
        ),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<1, 2>, "Equipped creature gets +1/+2.")],
    ..oracle::DWARVEN_SHORTSWORD
};

pub static EXTRA: &[&CardDef] = &[&tokens::DWARF];
