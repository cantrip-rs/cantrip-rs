use cantrip_api::*;
use cantrip_mechanics::{attach, targets, triggers};
use cantrip_oracle as oracle;

fn minus_one_power(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.power = chars.power.map(|v| v - 1);
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE_CARD_IN_A_GRAVEYARD),
    becomes_aura_enchant: Some(targets::CREATURE_PUT_ONTO_THE_BATTLEFIELD_WITH_THIS),
    abilities: &[
        triggered(triggers::THIS_ENTERS, attach::put_enchanted_creature_card_onto_the_battlefield::<false>, "When Animate Dead enters, if it's on the battlefield, it loses \"enchant creature card in a graveyard\" and gains \"enchant creature put onto the battlefield with Animate Dead.\" Return enchanted creature card to the battlefield under your control and attach Animate Dead to it.")
            .condition(triggers::this_is_on_the_battlefield),
        triggered(triggers::THIS_LEAVES, attach::sacrifice_creature_put_onto_the_battlefield_with_this, "When Animate Dead leaves the battlefield, that creature's controller sacrifices it."),
    ],
    statics: &[static_ability(Layer::PtModify, attach::attached_object, minus_one_power, "Enchanted creature gets -1/-0.")],
    ..oracle::ANIMATE_DEAD
};
