use cantrip_api::*;
use cantrip_mechanics::{effects, hand::exile_chosen_nonland_from_revealed_hand, ninjutsu, triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT};
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "RemoveAMenaceCounter",
    "remove a menace counter from it?",
    "When you do, that player reveals their hand and you choose a nonland card from it to exile",
    "Remove it",
    "Keep it",
);

/// "Whenever this creature deals combat damage to a player, you may remove a menace counter from it. When you do, ..." (CR 603.12):
/// not offered when it has no menace counter by now (or has left the battlefield: a counter cannot be removed from it). The
/// reflexive trigger is queued once the counter is off, about the player it dealt damage to.
fn remove_menace_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let me = item.object;
    if !cantrip_mechanics::targets::permanent(g, me) || g.obj(me).counters(CounterKind::Menace) == 0 {
        return Ok(());
    }
    if !effects::ask_may(g, item.controller, me, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    g.remove_counters(me, CounterKind::Menace, 1);
    let this = g.card_id(&CARD);
    g.reflexive_trigger(me, item.controller, AbilityRef::delayed(this, 0), item.subject);
    Ok(())
}

/// "That player reveals their hand and you choose a nonland card from it. Exile that card." Looking at the hand is part of the
/// ability, so the choice is asked whenever there is a nonland card to take.
fn reveal_and_exile(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = item.subject.player() else { return Ok(()) };
    exile_chosen_nonland_from_revealed_hand(g, item, player)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // "This creature enters with a menace counter on it" (CR 122.1b, 614.1c).
    enters: Some(effects::enters_with_a_counter::<{ CounterKind::Menace as usize }>),
    abilities: &[
        ninjutsu::ninjutsu(mana("{2}{B}"), "Ninjutsu {2}{B} ({2}{B}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)"),
        triggered(
            DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT,
            remove_menace_counter,
            "Whenever this creature deals combat damage to a player, you may remove a menace counter from it. When you do, that player reveals their hand and you choose a nonland card from it. Exile that card.",
        ),
    ],
    delayed: &[delayed_trigger(reveal_and_exile, "When you do, that player reveals their hand and you choose a nonland card from it. Exile that card.")],
    ..oracle::BITING_PALM_NINJA
};
