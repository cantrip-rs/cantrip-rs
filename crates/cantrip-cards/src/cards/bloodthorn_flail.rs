use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip_one_of};
use cantrip_mechanics::statics::plus_pt;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[equip_one_of(
        &[
            ExtraCost { text: "Pay {3}", cost: Cost::mana(mana("{3}")) },
            ExtraCost { text: "Discard a card", cost: discard_cards(1) },
        ],
        "Equip\u{2014}Pay {3} or discard a card",
    )],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<2, 1>, "Equipped creature gets +2/+1.")],
    ..oracle::BLOODTHORN_FLAIL
};
