use cantrip_api::*;
use cantrip_mechanics::{effects, power_up};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on Kang. Take an extra turn after this one. During that turn, power-up abilities can't be activated"
/// (CR 500.7, 602.5): "that turn" is the extra turn this ability created, named by the number `add_extra_turn` gives it, so
/// another extra turn (a second Kang's, a Time Walk's) and the rest of this turn are not under it. A player who has left the game
/// takes no turn (CR 800.4a), and nothing is barred.
fn conquer(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counter_on_this(g, item)?;
    g.end_sentence();
    let Some(turn) = g.add_extra_turn(item.controller) else { return Ok(()) };
    g.end_sentence();
    g.add_player_effect(item.controller, item.object, EffectKind::CantActivatePowerUp, Duration::EndOfExtraTurn(turn));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[power_up::ability(
        cost("{5}{U}{U}{U}"),
        conquer,
        "Power-up \u{2014} {5}{U}{U}{U}: Put a +1/+1 counter on Kang. Take an extra turn after this one. During that turn, power-up abilities can't be activated. (Activate each power-up ability only once. Reduce the cost by his mana cost if he entered this turn.)",
    )],
    ..oracle::KANG_THE_CONQUEROR
};
