use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Whenever this creature deals damage to a player, that player loses 1 life at the beginning of their next draw step
/// unless they pay {1} before that draw step." The trigger's resolution makes a delayed triggered ability (CR 603.7) for
/// that player's next draw step, the Asp's controller's (CR 603.7e), and an effect that lets that player pay {1} as a special
/// action any time they have priority before that draw step begins (CR 116.2c, 500.4), which stops that delayed ability from
/// triggering. Each damaging is its own pair: hit twice, the player pays twice or loses twice (the 2004-10-04 ruling).
fn loses_one_unless_they_pay(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = item.subject.player() else { return Ok(()) };
    let card = item.ability.card;
    g.delayed_trigger_unless(
        DelayedWhen::Step { step: Step::Draw, on_turn_of: Some(player) },
        item.object,
        item.controller,
        AbilityRef::delayed(card, 0),
        Target::Player(player),
        player,
        AbilityRef::delayed(card, 1),
        Duration::UntilNextStepOf { step: Step::Draw, player: Some(player) },
    );
    Ok(())
}

/// "That player loses 1 life at the beginning of their next draw step" (CR 603.7, 119.3): life lost, not damage, so it
/// does not trigger the Asp again (the 2004-10-04 ruling).
fn that_player_loses_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = item.subject.player() else { return Ok(()) };
    g.lose_life(player, 1);
    Ok(())
}

/// "Unless they pay {1} before that draw step" (CR 116.2c): paying is all the action does; the core then takes the
/// delayed ability away.
fn paid(_: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::DAMAGE, triggers::this_deals_damage_to_a_player),
        loses_one_unless_they_pay,
        "Whenever this creature deals damage to a player, that player loses 1 life at the beginning of their next draw step unless they pay {1} before that draw step.",
    )],
    delayed: &[
        delayed_trigger(that_player_loses_one, "that player loses 1 life at the beginning of their next draw step unless they pay {1} before that draw step."),
        ability(cost("{1}"), paid, "Pay {1} before your next draw step, and you don't lose 1 life at its beginning.").special_action(),
    ],
    ..oracle::NAFS_ASP
};
