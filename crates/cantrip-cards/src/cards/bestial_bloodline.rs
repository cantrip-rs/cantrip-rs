use cantrip_api::*;
use cantrip_mechanics::{attach, effects, statics, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[ability(cost("{4}{G}"), effects::return_this_to_hand, "{4}{G}: Return this card from your graveyard to your hand.").from(ZoneKind::Graveyard)],
    statics: &[static_ability(Layer::PtModify, attach::attached_object, statics::plus_pt::<2, 2>, "Enchanted creature gets +2/+2.")],
    ..oracle::BESTIAL_BLOODLINE
};
