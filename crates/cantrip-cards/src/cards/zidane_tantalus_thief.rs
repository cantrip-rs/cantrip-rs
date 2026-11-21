use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "When Zidane enters, gain control of target creature an opponent controls until end of turn. Untap it. It gains lifelink and
/// haste until end of turn.": Act of Treason's control, untap and haste (CR 613.1b, 611.2a, 702.10), then lifelink for the same
/// duration. A target that is no longer legal is no creature to take (CR 608.2b).
fn steal(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = effects::gain_control_untap_haste(g, item) else { return Ok(()) };
    g.end_sentence();
    effects::gain_lifelink_until_end_of_turn(g, item.object, creature);
    Ok(())
}

/// "Whenever an opponent gains control of a permanent from you" (CR 603.10d): `from` is the player who had it, the ability's
/// controller as it looks back on the change - Zidane himself being taken included, the Treasure going to the player who had him.
fn opponent_gains_control_from_you(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::ControlChanged { permanent, from, controller } if from == me.controller && controller != me.controller => Fired::about(Target::Object(permanent)),
        _ => None,
    }
}


pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            steal,
            "When Zidane enters, gain control of target creature an opponent controls until end of turn. Untap it. It gains lifelink and haste until end of turn.",
        )
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        triggered(
            when_asked(EventMask::CONTROL_CHANGED, opponent_gains_control_from_you),
            tokens::create_a_treasure,
            "Whenever an opponent gains control of a permanent from you, you create a Treasure token.",
        ),
    ],
    ..oracle::ZIDANE_TANTALUS_THIEF
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
