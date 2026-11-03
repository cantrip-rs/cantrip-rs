use cantrip_api::*;
use cantrip_mechanics::{statics, tokens, triggers};
use cantrip_oracle as oracle;

/// "Whenever another permanent you control leaves the battlefield during your turn" (CR 603.6c, 603.10a), looked back on: a
/// permanent other than Suki, under her controller's control as it last was on the battlefield, to whichever zone it went,
/// while it is her controller's turn.
fn another_permanent_you_control_leaves_on_your_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let left = triggers::permanent_you_control_left_on_your_turn(q, me, event)?;
    if left != me.source { Fired::PLAIN } else { None }
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::PtModify, statics::other_creatures_you_control, statics::plus_pt::<1, 0>, "Other creatures you control get +1/+0.")],
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, another_permanent_you_control_leaves_on_your_turn),
        tokens::create_an_ally,
        "Whenever another permanent you control leaves the battlefield during your turn, create a 1/1 white Ally creature token. This ability triggers only once each turn.",
    )
    .triggers_each_turn(1)],
    ..oracle::SUKI_COURAGEOUS_RESCUER
};

pub static EXTRA: &[&CardDef] = &[&tokens::ALLY];
