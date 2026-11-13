use cantrip_api::*;
use cantrip_mechanics::triggers::of_subtype_you_control_dies;
use cantrip_mechanics::{effects, lands, statics};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        statics::attacking_of_subtype_you_control::<{ lands::sub(Subtype::Vampire) }>,
        statics::have_deathtouch_and_lifelink,
        "Attacking Vampires you control have deathtouch and lifelink.",
    )],
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, of_subtype_you_control_dies::<{ lands::sub(Subtype::Vampire) }>),
        |g, item| effects::may_pay_to_draw(g, item, life(2)),
        "Whenever a Vampire you control dies, you may pay 2 life. If you do, draw a card.",
    )],
    ..oracle::CROSSWAY_TROUBLEMAKERS
};
