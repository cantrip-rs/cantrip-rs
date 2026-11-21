use cantrip_api::*;
use cantrip_mechanics::{earthbend, lands, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Ally) }>),
        earthbend::effect::<1>,
        "Whenever another Ally you control enters, earthbend 1. (Target land you control becomes a 0/0 creature with haste that's still a land. Put a +1/+1 counter on it. When it dies or is exiled, return it to the battlefield tapped.)",
    )
    .targets(&[earthbend::TARGET])],
    delayed: &[earthbend::RETURN],
    ..oracle::HARU_HIDDEN_TALENT
};
