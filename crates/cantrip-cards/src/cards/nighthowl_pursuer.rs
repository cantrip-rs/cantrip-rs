use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_oracle as oracle;

/// "This creature gets +2/+2 until end of turn" (CR 611.2): a continuous effect, which does nothing if the creature has left
/// the battlefield by now.
fn pump_self(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::pump(g, item.object, item.object, 2, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        pump_self,
        "Ferocious \u{2014} Whenever this creature attacks while you control a creature with power 4 or greater, this creature gets +2/+2 until end of turn.",
    )
    .only_if(cantrip_mechanics::ferocious::you_control_asked)],
    ..oracle::NIGHTHOWL_PURSUER
};
