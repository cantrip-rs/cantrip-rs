use cantrip_api::*;
use cantrip_mechanics::{effects, history, targets, tokens, triggers::YOUR_UPKEEP};
use cantrip_oracle as oracle;

/// "Target creature an opponent controls with mana value 3 or less" (CR 115.1, 202.3: an {X} in a cost on the battlefield
/// is 0): its controller is not the ability's controller, and its mana value is read as it is chosen and again as the
/// mode resolves (CR 608.2b).
fn creature_an_opponent_controls_with_mana_value_3_or_less(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::creature_with_mana_value_at_most::<3>(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) != asker.controller)
}

/// "Up to one target creature an opponent controls with mana value 3 or less", for the Hail mode: fewer than one (or none)
/// is chosen as readily as one.
static HAIL_TARGET: TargetSpec = target(creature_an_opponent_controls_with_mana_value_3_or_less).optional();

/// "Transport — Create two 1/1 red Officer creature tokens": both at once, as one event (CR 101.4, 608.2c).
fn transport(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::OFFICER, 2)?;
    Ok(())
}

/// "Abandon Ship — You lose the game" (CR 104.3a).
fn abandon_ship(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.lose_game(item.controller);
    Ok(())
}

/// "Choose one that hasn't been chosen" (CR 700.2, 700.2b): the modes this Scenario has chosen before, whoever chose them and
/// whatever became of those abilities, are read from the event history (`history::mode_not_chosen_before`). Hail's target is
/// optional, so it is always a mode that can be chosen - with no legal target, no target question is asked (CR 115.1d) -
/// and with only "you lose the game" left that mode must be chosen, while with no mode left the ability is removed from
/// the stack with no effect (CR 603.3c).
static SCENARIO: ModalDef = modes(&[
    mode(
        effects::return_first_target_to_hand,
        "Hail — Return up to one target creature an opponent controls with mana value 3 or less to its owner's hand.",
    )
    .targets(&[HAIL_TARGET]),
    mode(|g, item| effects::draw(g, item, 2), "Sensors — Draw two cards."),
    mode(transport, "Transport — Create two 1/1 red Officer creature tokens."),
    mode(abandon_ship, "Abandon Ship — You lose the game."),
])
.allowed(history::mode_not_chosen_before);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        YOUR_UPKEEP,
        &SCENARIO,
        "At the beginning of your upkeep, choose one that hasn't been chosen — Hail — Return up to one target creature an opponent controls with mana value 3 or less to its owner's hand. • Sensors — Draw two cards. • Transport — Create two 1/1 red Officer creature tokens. • Abandon Ship — You lose the game.",
    )],
    ..oracle::KOBAYASHI_MARU_SCENARIO
};

pub static EXTRA: &[&CardDef] = &[&tokens::OFFICER];
