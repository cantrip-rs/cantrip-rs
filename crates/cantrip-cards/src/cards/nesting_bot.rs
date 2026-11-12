use cantrip_api::*;
use cantrip_mechanics::max_speed::itself_at_max_speed;
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

/// "When this creature dies, create a 1/1 colorless Servo artifact creature token." The token is made for the ability's
/// controller, who is the Bot's controller as it died (CR 603.3a, 603.10a), not its owner.
fn servo(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::SERVO, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[triggered(THIS_DIES, servo, "When this creature dies, create a 1/1 colorless Servo artifact creature token.")],
    statics: &[static_ability(Layer::PtModify, itself_at_max_speed, plus_pt::<1, 0>, "Max speed \u{2014} This creature gets +1/+0.")],
    ..oracle::NESTING_BOT
};

pub static EXTRA: &[&CardDef] = &[&tokens::SERVO];
