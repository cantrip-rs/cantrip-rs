use cantrip_api::*;
use cantrip_mechanics::statics::permanent_entering_triggers_again;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Copy target activated or triggered ability you control. You may choose new targets for the copy." (CR 707.10, 707.10c)
/// The copy is neither activated nor triggered; with the ability no longer there nothing happens (CR 608.2b).
fn copy_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.copy_ability(item.target(0), item.controller, true)?;
    Ok(())
}

/// The Adventure (CR 715).
pub static VANTRESS_VISIONS: CardDef = CardDef {
    spell: spell(&[targets::YOUR_ACTIVATED_OR_TRIGGERED_ABILITY], copy_it),
    ..oracle::VANTRESS_VISIONS
};

/// "If a permanent entering causes a triggered ability of a permanent you control to trigger, that ability triggers an
/// additional time." (CR 603.2d)
pub static CARD: CardDef = CardDef {
    additional_triggers: &[permanent_entering_triggers_again],
    adventure: Some(&VANTRESS_VISIONS),
    ..oracle::VIRTUE_OF_KNOWLEDGE
};

pub static EXTRA: &[&CardDef] = &[&VANTRESS_VISIONS];
