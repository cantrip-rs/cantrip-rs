//! "At the beginning of the next end step, [do something] to it" (CR 603.7): a delayed triggered ability a spell or ability
//! sets up about one object. The card lists the ability in its `delayed`; these register it and are the bodies it names.

use cantrip_api::*;

/// The card whose `delayed` list the resolving ability's delayed ability is in: the card the ability being resolved belongs to
/// (`item.ability`). That is the source's card for its own ability, the card it copies for a copy (CR 707.2), and the card that
/// granted it for an ability the source has been given (CR 113.6, 613.1f: Agatha's Soul Cauldron lending an exiled creature's
/// ability, Way of the Cryomancer's planeswalkers), whatever the source is.
fn ability_card(_: &dyn Game, item: &StackItem) -> CardId {
    item.ability.card
}

/// Sets up the card's delayed ability number `ability` (an index into its `delayed` list) to trigger at the beginning of
/// the next end step (CR 603.7), about `subject`, controlled by the resolving ability's controller. Sneak Attack, Through
/// the Breach, Flickerwisp and Daretti, Scrap Savant's emblem use it. The ability is the resolving ability's card:
/// the card it copies, for a copy (Phantasmal Image as Flickerwisp, CR 707.2).
pub fn at_next_end_step(g: &mut dyn Game, item: &StackItem, ability: u8, subject: Target) {
    let this = ability_card(g, item);
    g.delayed_trigger(
        DelayedWhen::Step { step: Step::End, on_turn_of: None },
        item.object,
        item.controller,
        AbilityRef::delayed(this, ability),
        subject,
    );
}

/// Sets up the card's delayed ability number `ability` to trigger at the beginning of the end of combat step (CR 513.1, 603.7),
/// about `subject`, controlled by the resolving ability's controller. Cockatrice and Abomination ("destroy that creature at end of
/// combat") use it, with `DESTROY_THAT_CREATURE_AT_END_OF_COMBAT`.
pub fn at_end_of_combat(g: &mut dyn Game, item: &StackItem, ability: u8, subject: Target) {
    let this = ability_card(g, item);
    g.delayed_trigger(
        DelayedWhen::Step { step: Step::EndOfCombat, on_turn_of: None },
        item.object,
        item.controller,
        AbilityRef::delayed(this, ability),
        subject,
    );
}

/// "Destroy that creature at end of combat" (CR 603.7c): the trigger's effect, setting up `DESTROY_THAT_CREATURE_AT_END_OF_COMBAT`
/// (the card's delayed ability number 0) about the creature the trigger is about.
pub fn destroy_that_creature_at_end_of_combat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    at_end_of_combat(g, item, 0, item.subject);
    Ok(())
}

/// "Destroy it": the delayed ability's subject, if it is still the object it was (CR 603.7c, 400.7), even if it is no longer a
/// creature (CR 701.8a). Cockatrice and Abomination print "destroy that creature at end of combat".
pub fn destroy_subject(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(it) = item.subject {
        g.destroy(it);
    }
    Ok(())
}

/// The delayed ability of "destroy that creature at end of combat" (CR 603.7c), the body of `destroy_subject`; a card lists it
/// among its `delayed` abilities and names its index in `at_end_of_combat`.
pub const DESTROY_THAT_CREATURE_AT_END_OF_COMBAT: AbilityDef = destroy_that_at_end_of_combat("Destroy that creature at end of combat.");

/// The same delayed ability with a card's own wording of it, for a card whose text names the object another way ("destroy that Wall
/// at end of combat": Battering Ram), so the ability's text is the card's Oracle text.
pub const fn destroy_that_at_end_of_combat(text: &'static str) -> AbilityDef {
    delayed_trigger(destroy_subject, text)
}

/// "Exile [the target in slot `slot`]. Return that card to the battlefield under its owner's control at the beginning of the next
/// end step" (CR 603.7): exiles the target and sets up the card's delayed ability number `ability` about the card exiled, but only
/// if it is in exile: a token ceases to exist (CR 111.7, 400.7), and an exile replaced to somewhere else leaves nothing to
/// return. Nothing at all happens if the target is no longer a legal target as the ability resolves, or none was chosen ("up to
/// one"). Flickerwisp, Ennis, Debate Moderator, S.H.I.E.L.D. Flying Car and Wiccan, Rising Magician print it.
pub fn exile_target_then_return_at_next_end_step(g: &mut dyn Game, item: &StackItem, slot: usize, ability: u8) {
    let Some(object) = crate::targets::object(g, item, slot) else { return };
    // Every card a merged permanent became is returned (CR 730.3c): the delayed ability carries them all
    // (`return_subject_to_battlefield`). It is created whatever was exiled (CR 603.7a), a token too, which will have ceased to
    // exist by then (CR 111.7), so the ability does nothing as it resolves (CR 603.7c).
    let exiled: Vec<ObjectId> = g.exile_all(&[object]).into_iter().filter(|&c| g.obj(c).zone.is(ZoneKind::Exile)).collect();
    match exiled[..] {
        [] => {}
        [one] => at_next_end_step(g, item, ability, Target::Object(one)),
        _ => at_next_end_step_about(g, item, ability, &exiled),
    }
}

/// Sets up the card's delayed ability number 0 to trigger "this turn" (CR 603.7b): each time its `listens`/`matches`
/// accept an event, until the turn ends, controlled by the resolving ability's controller. `source` is the object the
/// ability's `matches` is asked of: the spell for "whenever a creature you control dies this turn", the creature it
/// is about for "when it dies this turn". A `matches` that says "you" is written `when_asked`: the ability's controller it
/// is told is this resolving ability's controller (CR 603.7d-e), whoever controls `source` later. Forth Eorlingas!, Glimpse of Nature, Rite of Harmony, Showdown of the Skalds,
/// Thunder of Unity, Warhost's Frenzy, Okoye and Turn Inside Out use it. The ability is the card's as the resolving ability has
/// it, the card it copies for a copy.
pub fn this_turn(g: &mut dyn Game, item: &StackItem, source: ObjectId) {
    let this = ability_card(g, item);
    g.delayed_trigger(DelayedWhen::ThisTurn, source, item.controller, AbilityRef::delayed(this, 0), Target::None);
}

/// Watch a particular object for events this turn while keeping the creating spell or ability as the source
/// (CR 603.7b, 603.7d-e). A Good Day to Die and End Blaze Epiphany both watch a creature dying this turn.
/// The trigger receives its event's `Fired` payload; when it supplies no subject, the watched object is the subject.
/// The captured words belong to the creating ability.
pub fn this_turn_watching(g: &mut dyn Game, item: &StackItem, watched: ObjectId) {
    let card = ability_card(g, item);
    g.delayed_trigger(DelayedWhen::ThisTurnWatching(watched), item.object, item.controller, AbilityRef::delayed(card, 0), Target::None);
}

/// Sets up the card's delayed ability number `ability` to trigger "when you next [event] this turn" (CR 603.7b): once, the
/// first time its `listens`/`matches` accept an event this turn, controlled by the resolving ability's controller; it ends
/// with the turn if it never does. `source` is the object its `matches` is asked of, as for `this_turn`. Each call is a
/// separate ability, so two activations both trigger on the same next event. The ability is the resolving ability's card
/// (the card it copies, for a copy). Loki Laufeyson uses it; Doublecast, Teach by Example and Galvanic Iteration print it.
pub fn next_this_turn(g: &mut dyn Game, item: &StackItem, ability: u8, source: ObjectId) {
    let card = ability_card(g, item);
    g.delayed_trigger(DelayedWhen::NextThisTurn, source, item.controller, AbilityRef::delayed(card, ability), Target::None);
}

/// `at_next_end_step_about` for "at the beginning of your next end step" (CR 603.7): the first end step of the ability's
/// controller's turn that has not begun yet, which is this turn's when the ability resolves before it. Dragonhawk, Fate's
/// Tempest uses it for the cards it exiled.
pub fn at_your_next_end_step_about(g: &mut dyn Game, item: &StackItem, ability: u8, created: &[ObjectId]) {
    let card = ability_card(g, item);
    g.delayed_trigger_about(
        DelayedWhen::Step { step: Step::End, on_turn_of: Some(item.controller) },
        item.object,
        item.controller,
        AbilityRef::delayed(card, ability),
        created,
    );
}

/// Sets up the card's delayed ability number `ability` to trigger at the beginning of the next end step (CR 603.7), carrying
/// the objects `created` (CR 603.7c) to the moment it triggers, where `event_objects` of its stack item gives them back.
/// The ability is the card as the resolving ability has it: its card, or the card it copies or was granted by. Mobilize's Warriors and Dalkovan Encampment's
/// use it with `sacrifice_created`.
pub fn at_next_end_step_about(g: &mut dyn Game, item: &StackItem, ability: u8, created: &[ObjectId]) {
    let card = ability_card(g, item);
    g.delayed_trigger_about(
        DelayedWhen::Step { step: Step::End, on_turn_of: None },
        item.object,
        item.controller,
        AbilityRef::delayed(card, ability),
        created,
    );
}

/// `at_next_end_step_about` for "at the beginning of the next upkeep" (CR 503.1, 603.7): the next upkeep step to begin,
/// whoever's turn it is in, carrying the objects `created` (CR 603.7c). Firion, Wild Rose Warrior sacrifices its token then
/// (`sacrifice_created`); The Eagles Are Coming! carries the creatures it returned.
pub fn at_next_upkeep_about(g: &mut dyn Game, item: &StackItem, ability: u8, created: &[ObjectId]) {
    let card = ability_card(g, item);
    g.delayed_trigger_about(
        DelayedWhen::Step { step: Step::Upkeep, on_turn_of: None },
        item.object,
        item.controller,
        AbilityRef::delayed(card, ability),
        created,
    );
}

/// "Destroy it at the beginning of the next end step": destroy the carried objects that are still
/// permanents (CR 603.7c, 400.7, 701.8). Glyph of Destruction and Old Hob, Alleycat Blues use it.
pub fn destroy_carried(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects = g.event_objects(item).to_vec();
    g.destroy_all(&objects);
    Ok(())
}

/// "Sacrifice them": the objects the delayed ability carries (`at_next_end_step_about`) that are still on the battlefield
/// as the objects they were (CR 603.7c, 400.7), sacrificed in one action by this ability's controller (CR 603.7d-e), who
/// can't sacrifice one another player controls now (CR 701.21a).
pub fn sacrifice_created(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let created = g.event_objects(item).to_vec();
    g.sacrifice(item.controller, &created);
    Ok(())
}

/// The delayed ability of "sacrifice them at the beginning of the next end step" (CR 603.7c), the body of
/// `sacrifice_created`; a card lists it among its `delayed` abilities and names its index in `at_next_end_step_about`.
pub const SACRIFICE_CREATED: AbilityDef = delayed_trigger(sacrifice_created, "Sacrifice them at the beginning of the next end step.");

/// "Sacrifice it": the delayed ability's subject, if it is still the object it was on the battlefield (CR 603.7c, 400.7)
/// and this ability's controller (CR 603.7d-e) still controls it (CR 701.21a). Sneak Attack and Through the Breach print it
/// at the next end step.
pub fn sacrifice_subject(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(it) = item.subject {
        g.sacrifice(item.controller, &[it]);
    }
    Ok(())
}

/// "Exile it": the delayed ability's subject, if it is still the object it was on the battlefield (CR 603.7c, 400.7).
/// Shallow Grave, Goryo's Vengeance and Corpse Dance print it at the next end step. A delayed ability that carries several
/// objects (`at_next_end_step_about`: Vindictive Triumph, whose "it" is each card of a merged permanent it returned, CR 730.3c)
/// exiles each that is still the object it was on the battlefield, together.
pub fn exile_subject(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if item.subject == Target::None {
        let carried: Vec<ObjectId> = g.event_objects(item).iter().copied().filter(|&c| is_live(g, c) && g.is_on_battlefield(c)).collect();
        g.exile_all(&carried);
        return Ok(());
    }
    if let Target::Object(it) = item.subject
        && is_live(g, it)
        && g.is_on_battlefield(it)
    {
        g.exile(it);
    }
    Ok(())
}

/// "Return that card to the battlefield under its owner's control": the delayed ability's subject, if it is still the
/// card it was when the ability was set up (a new object if it left its zone, CR 603.7c, 400.7). Flickerwisp and Daretti,
/// Scrap Savant's emblem print it at the next end step.
pub fn return_subject_to_battlefield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    // The cards a merged permanent became, carried together (CR 730.3c, `exile_target_then_return_at_next_end_step`): each
    // still in exile returns, together.
    if item.subject == Target::None && !g.event_objects(item).is_empty() {
        return return_exiled_to_battlefield(g, item);
    }
    let Target::Object(card) = item.subject else { return Ok(()) };
    if !is_live(g, card) {
        return Ok(());
    }
    let owner = g.obj(card).owner;
    g.put_onto_battlefield(card, owner, false)?;
    Ok(())
}

/// "Return those cards to the battlefield under their owners' control": the cards the delayed ability carries
/// (`at_next_end_step_about`) that are still the cards they were and still in exile (CR 603.7c, 400.7: one that left exile is a
/// new object, a token has ceased to exist, CR 111.7), put onto the battlefield together (CR 101.4), each under its owner's
/// control (CR 110.2a). Yorion, Sky Nomad and Hide on the Ceiling print it at the next end step.
pub fn return_exiled_to_battlefield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards = crate::library::still_exiled(g, g.event_objects(item));
    let entering: Vec<(ObjectId, PlayerId)> = cards.into_iter().map(|c| (c, g.obj(c).owner)).collect();
    g.put_all_onto_battlefield(&entering)?;
    Ok(())
}

/// "Return it to your hand": the delayed ability's subject, if it is still the object it was on the battlefield (CR 603.7c,
/// 400.7); it goes to its owner's hand (CR 400.3), and `return_to_hand` leaves an object that has changed zones alone.
/// Anzrag's Rampage prints it at the next end step.
pub fn return_subject_to_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(it) = item.subject {
        g.return_to_hand(&[it]);
    }
    Ok(())
}

/// "Take an extra turn after this one. At the beginning of that turn's end step, you lose the game." (CR 500.7, 603.7): the
/// extra turn is the resolving spell's controller's, and the delayed ability (the card's `delayed` number `ability`,
/// `LOSE_AT_THAT_END_STEP`) waits for that turn's end step alone. If that turn is skipped (CR 500.11; Final Fortune's ruling:
/// "If you end up skipping the extra turn that is gained, you do not lose the game") or ends without an end step (Sundial of
/// the Infinite, CR 603.7, 724.1e), it never triggers. Final Fortune and Last Chance print it.
pub fn extra_turn_then_lose(g: &mut dyn Game, item: &StackItem, ability: u8) {
    let Some(turn) = g.add_extra_turn(item.controller) else { return };
    let card = ability_card(g, item);
    g.delayed_trigger(
        DelayedWhen::ExtraTurnStep { step: Step::End, turn },
        item.object,
        item.controller,
        AbilityRef::delayed(card, ability),
        Target::None,
    );
}

/// "You lose the game" (CR 104.3e): the body of `LOSE_AT_THAT_END_STEP`.
fn lose_the_game(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.lose_game(item.controller);
    Ok(())
}

/// The delayed ability `extra_turn_then_lose` sets up; a card lists it among its `delayed` abilities.
pub const LOSE_AT_THAT_END_STEP: AbilityDef = delayed_trigger(lose_the_game, "At the beginning of that turn's end step, you lose the game.");

/// `at_next_end_step_about` for "at the beginning of the [step] on your next turn" (CR 603.7): that step of the ability's
/// controller's first turn to begin after this one, never this turn's, even when this turn is theirs. If that turn ends
/// without the step, the ability never triggers (`DelayedWhen::NextTurnStep`). Kav Landseeker prints it for the end step,
/// Meandering Towershell for the declare attackers step.
pub fn at_step_on_your_next_turn_about(g: &mut dyn Game, item: &StackItem, step: Step, ability: u8, created: &[ObjectId]) {
    let card = ability_card(g, item);
    let after = g.state().turn.number;
    g.delayed_trigger_about(
        DelayedWhen::NextTurnStep { step, player: item.controller, after },
        item.object,
        item.controller,
        AbilityRef::delayed(card, ability),
        created,
    );
}

/// Sets up the card's delayed ability number `ability` to trigger "at the beginning of your next main phase" (CR 603.7), about
/// `subject`, controlled by the resolving ability's controller. Which step that is depends on when this resolves: from the
/// controller's own precombat main phase or combat it is that turn's postcombat main phase, from anywhere else the
/// precombat main phase of their next turn (this turn's, before it has begun). Mana Drain and Mana Sculpt use it.
pub fn at_your_next_main_phase(g: &mut dyn Game, item: &StackItem, ability: u8, subject: Target) {
    let turn = g.state().turn;
    let step = if turn.active == item.controller
        && (Step::PrecombatMain as u8..Step::PostcombatMain as u8).contains(&(turn.step() as u8))
    {
        Step::PostcombatMain
    } else {
        Step::PrecombatMain
    };
    let this = ability_card(g, item);
    g.delayed_trigger(
        DelayedWhen::Step { step, on_turn_of: Some(item.controller) },
        item.object,
        item.controller,
        AbilityRef::delayed(this, ability),
        subject,
    );
}
