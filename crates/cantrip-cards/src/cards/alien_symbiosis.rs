use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, cast_from_your_graveyard, lands, statics, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    statics: &[
        static_ability(Layer::Type, attached_object, statics::also_a::<{ lands::sub(Subtype::Symbiote) }>, "Enchanted creature is a Symbiote in addition to its other types."),
        static_ability(Layer::Ability, attached_object, statics::have_menace, "Enchanted creature has menace.").continues(),
        static_ability(Layer::PtModify, attached_object, statics::plus_pt::<1, 1>, "Enchanted creature gets +1/+1.").continues(),
    ],
    // "You may cast this card from your graveyard by discarding a card in addition to paying its other costs" (CR 113.6e-f,
    // 601.3, 118.8): cast for its mana cost or an alternative cost it has, at the time an Aura may be cast (the ruling:
    // "You must follow all normal timing rules"), with a card discarded on top.
    exile_casts: &[ExileCast {
        text: "Cast from your graveyard by discarding a card",
        additional: Cost { discard: Discards::one(1, None), ..Cost::NONE },
        ..cast_from_your_graveyard::PERMISSION
    }],
    ..oracle::ALIEN_SYMBIOSIS
};
