use cantrip_api::*;
use cantrip_mechanics::triggers::{activated_ability_on_stack, activation};
use cantrip_mechanics::delayed;
use cantrip_oracle as oracle;

/// "When you next activate an exhaust ability that isn't a mana ability" (CR 702.177a, 602.2, 605.1a, 603.2): an ability of
/// this ability's controller that has exhaust and uses the stack, about that ability on the stack (to copy it).
fn you_activate_an_exhaust_ability_that_isnt_a_mana_ability(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let (_, player, def) = activation(q, event)?;
    if player != me.controller || !def.exhaust || def.is_mana_ability() {
        return None;
    }
    Fired::about(activated_ability_on_stack(q, event)?)
}

/// "Copy it. You may choose new targets for the copy." (CR 707.10, 707.10c) The ability the trigger is about, copied even if
/// it has left the stack (as it last was there).
fn copy_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.copy_ability(item.subject, item.controller, true)?;
    Ok(())
}

/// "When you next activate an exhaust ability that isn't a mana ability this turn" (CR 603.7b): sets up delayed triggered
/// ability number 0, which triggers once on the next such activation this turn and ends with the turn.
fn watch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    delayed::next_this_turn(g, item, 0, item.object);
    Ok(())
}

const COPY_IT: AbilityDef = triggered(
    when_asked(EventMask::ABILITY_ACTIVATED, you_activate_an_exhaust_ability_that_isnt_a_mana_ability),
    copy_it,
    "When you next activate an exhaust ability that isn't a mana ability this turn, copy it. You may choose new targets for the copy.",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add_n(Mana::C, 2).only_for(ManaRestriction::ABILITIES), "{T}: Add {C}{C}. Spend this mana only to activate abilities."),
        ability(
            cost("{2}, {T}"),
            watch,
            "{2}, {T}: When you next activate an exhaust ability that isn't a mana ability this turn, copy it. You may choose new targets for the copy.",
        ),
    ],
    delayed: &[COPY_IT],
    ..oracle::PIT_AUTOMATON
};
