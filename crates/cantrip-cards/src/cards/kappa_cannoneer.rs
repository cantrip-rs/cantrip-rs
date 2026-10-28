use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::{ANOTHER_ARTIFACT_YOU_CONTROL_ENTERS, THIS_ENTERS, ward};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on this creature. It can't be blocked this turn."
fn grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if targets::permanent(g, item.object) {
        g.add_counters(item.object, CounterKind::PlusOne, 1);
        g.add_effect(item.object, item.object, EffectKind::CantBeBlocked, Duration::EndOfTurn);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<4>("Ward {4} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {4}.)"),
        triggered(THIS_ENTERS, grow, "Whenever Kappa Cannoneer enters, put a +1/+1 counter on it. It can't be blocked this turn."),
        triggered(ANOTHER_ARTIFACT_YOU_CONTROL_ENTERS, grow, "Whenever another artifact you control enters, put a +1/+1 counter on Kappa Cannoneer. It can't be blocked this turn."),
    ],
    ..oracle::KAPPA_CANNONEER
};
