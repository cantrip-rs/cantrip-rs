use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// The controller may pay {1} to gain 1 life (CR 608.2d, 119.3).
fn pay_to_gain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_gain_life(g, item, cost("{1}"))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::type_you_control_to_graveyard::<{ CardType::Artifact as u8 }>),
        pay_to_gain,
        "Whenever an artifact you control is put into a graveyard from the battlefield, you may pay {1}. If you do, you gain 1 life.",
    )],
    ..oracle::TABLET_OF_EPITYR
};
