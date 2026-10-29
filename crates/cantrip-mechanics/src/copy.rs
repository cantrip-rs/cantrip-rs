//! Copy effects many cards print (CR 707).

use cantrip_api::*;

/// What a permanent that may enter as a copy asks (CR 707.2): yes, and what it copies is chosen next.
const COPY_AS_ENTERS: Question =
    Question::new("CopyAsEnters", "enter as a copy?", "You choose what it copies next. If you don't, it enters as itself", "Choose a copy", "Enter as itself");

/// "You may have this [permanent] enter as a copy of any [permanent that passes `eligible`] on the battlefield, except
/// [`exception`]" (CR 614.1c, 707.2, 707.9): a choice, not a target (CR 608.2d), so hexproof and protection narrow nothing
/// and an opponent's permanent is as available as your own. With nothing to copy nothing is asked, and with one candidate which to copy is not. The exception becomes part of the copy's own copiable values (CR 707.9b),
/// so a later copy of it has it too. Use it as `CardDef::enters_as_copy` (CR 616.1c). Phantasmal Image (a creature; an Illusion with a
/// sacrifice ability) and Phyrexian Metamorph (an artifact or a creature; an artifact too) print it.
pub fn enter_as_copy(
    g: &mut dyn Game,
    entry: &mut Entering,
    eligible: fn(&Chars) -> bool,
    exception: fn(&dyn Query, Copiable) -> Copiable,
) -> Result<(), Suspend> {
    enter_as_copy_asking(g, entry, eligible, exception, &COPY_AS_ENTERS)
}

/// `enter_as_copy`, asked in the card's own words: Vesuva's "enter tapped as a copy", where what the copy does as it enters
/// is part of the choice. Whether it enters as a copy is `entry.copy` afterwards.
pub fn enter_as_copy_asking(
    g: &mut dyn Game,
    entry: &mut Entering,
    eligible: fn(&Chars) -> bool,
    exception: fn(&dyn Query, Copiable) -> Copiable,
    question: &'static Question,
) -> Result<(), Suspend> {
    let entering = entry.object;
    let candidates: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| id != entering && eligible(&g.chars(id)))
        .collect();
    enter_as_copy_of_one_of(g, entry, &candidates, exception, question)
}

/// `enter_as_copy_asking` over the permanents the caller has already chosen: for a card whose "any creature on the
/// battlefield" is narrowed by something the entering permanent knows (Mockingbird's mana value no greater than the mana
/// spent to cast it), which a `fn(&Chars)` filter cannot see. `candidates` must not hold the entering permanent. It is the body of
/// `enter_as_copy_asking`, so every `enter_as_copy` card goes through it (Clone, Evil Twin, Phantasmal Image, Phyrexian Metamorph, Omni-Changeling,
/// Vesuva); Mockingbird calls it directly.
pub fn enter_as_copy_of_one_of(
    g: &mut dyn Game,
    entry: &mut Entering,
    candidates: &[ObjectId],
    exception: fn(&dyn Query, Copiable) -> Copiable,
    question: &'static Question,
) -> Result<(), Suspend> {
    let entering = entry.object;
    if candidates.is_empty() {
        return Ok(());
    }
    let controller = g.obj(entering).controller;
    if g.ask(controller, RequestKind::May { source: entering, purpose: Purpose::Ask(question) })? != Answer::Yes {
        return Ok(());
    }
    let chosen = if candidates.len() == 1 {
        candidates[0]
    } else {
        let ask = RequestKind::ChooseTarget { label: "", slot_player: None, together: None,
            source: entering,
            purpose: Purpose::CopyWhich,
            candidates: candidates.iter().map(|&id| Target::Object(id)).collect(),
            optional: false,
            least: 1,
            count: 1,
            any_number: false,
        };
        match g.ask(controller, ask)? {
            Answer::Targets(t) => t.first().and_then(|t| t.object()).unwrap_or(candidates[0]),
            _ => candidates[0],
        }
    };
    // The copied object's copiable values, its own copy effect's exceptions included (CR 707.2, 707.9b), and this copy's
    // exception on top.
    let values = g.copiable_values(chosen);
    entry.copy = Some(exception(g, values));
    Ok(())
}

/// The intervening "if it isn't a token" of "When this creature dies, if it isn't a token, create a token that's a copy
/// of it" (CR 603.4, 111.7): judged on the creature as it last existed on the battlefield (CR 603.10a). Use as
/// `TriggeredDef::condition` of a dies ability, which looks back. Sphinx of False Conclusions and Vaultborn Tyrant print it.
pub fn not_a_token(q: &dyn Query, asker: &Asker) -> bool {
    !q.obj(asker.source).token
}

/// "Create a token that's a copy of it" (CR 707.2, 111.1) for the ability's controller: the copiable values of the
/// ability's object as it last existed, with `exception` applied for the copy's exceptions (CR 707.9b), as
/// `enter_as_copy` takes them. Sphinx of False Conclusions copies as is; Vaultborn Tyrant's copy is an artifact in
/// addition to its other types.
pub fn create_token_copy_of_it(
    g: &mut dyn Game,
    item: &StackItem,
    exception: fn(&dyn Query, Copiable) -> Copiable,
) -> Result<(), Suspend> {
    let values = g.copiable_values(item.object);
    let values = exception(g, values);
    g.create_token_copies_of(values, item.controller, 1)?;
    Ok(())
}

/// "Create a copy of [an object] in exile. You may cast the copy without paying its mana cost" (CR 707.12, 118.9, 608.2g): the
/// copy is of `of` as it last existed, wherever it is now, owned by `player`. It is cast while the effect resolves, whatever its
/// timing (CR 608.2g), if it has a legal target and `player` says yes; one that is not cast, because it could not be or was not
/// wanted, stays in exile, where it ceases to exist (CR 707.10a, 704.5e). Paradigm's recurring copy and Shiko, Paragon of the Way
/// print it.
pub fn may_cast_a_copy_without_paying(g: &mut dyn Game, player: PlayerId, of: ObjectId) -> Result<(), Suspend> {
    let copy = g.create_copy_in_exile(of, player);
    if g.could_cast_during_resolution_by(copy, player) && g.ask(player, RequestKind::May { source: copy, purpose: Purpose::CastCopyFree })? == Answer::Yes {
        g.cast_during_resolution_as(copy, CastTerms { instead: Some(ManaCost::ZERO), caster: Some(player), ..CastTerms::default() })?;
    }
    Ok(())
}

/// "Create a token that's a copy of target [permanent]" (CR 707.2, 111.1): the copiable values of the first target, for
/// the ability's or spell's controller. Nothing happens if that target is no longer legal (CR 608.2b). Esika's Chariot,
/// Octomancer, Three Steps Ahead and Caretaker's Talent print it.
pub fn create_token_copy_of_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(target) = crate::targets::object(g, item, 0) {
        g.create_token_copies(target, item.controller, 1)?;
    }
    Ok(())
}

/// "Create a token that's a copy of [the target], except it has haste. Sacrifice it at the beginning of the next end step"
/// (CR 707.2, 707.9a, 111.1, 603.7): the first target's copiable values as the ability resolves, with haste as the
/// exception (so a copy of the token has haste too, and is not sacrificed: the rulings), for the ability's controller. The
/// delayed ability is the `delayed` list's ability number `ability` of `card` (the face that prints the ability: Reflection
/// of Kiki-Jiki is a back face), about every token the resolution made (CR 603.7c: a replacement that makes more makes each
/// of them sacrificed). Nothing if the target is no longer legal (CR 608.2b). Kiki-Jiki, Mirror Breaker and Reflection of
/// Kiki-Jiki print it.
pub fn create_hasty_token_copy_sacrificed_at_end_step(g: &mut dyn Game, item: &StackItem, card: CardId, ability: u8) -> Result<(), Suspend> {
    let Some(creature) = crate::targets::object(g, item, 0) else { return Ok(()) };
    let values = g.copiable_values(creature).with_keyword(Keyword::Haste);
    let tokens = g.create_token_copies_of(values, item.controller, 1)?;
    g.delayed_trigger_about(
        DelayedWhen::Step { step: Step::End, on_turn_of: None },
        item.object,
        item.controller,
        AbilityRef::delayed(card, ability),
        &tokens,
    );
    Ok(())
}

/// The delayed ability of `create_hasty_token_copy_sacrificed_at_end_step`, for the card's `delayed` list: "Sacrifice it",
/// each token it carries that is still on the battlefield under its controller (`delayed::sacrifice_created`).
pub const SACRIFICE_IT_AT_END_STEP: AbilityDef =
    delayed_trigger(crate::delayed::sacrifice_created, "Sacrifice it at the beginning of the next end step.");

/// "Copy that spell. You may choose new targets for the copy" (CR 707.10, 707.10c) for a trigger about the spell that was cast
/// (`Fired::about` the spell): the copy is not cast, and is made from its last-known information if the spell has left the stack
/// (CR 608.2h). Leyline of Resonance, Loki Laufeyson and Jace Reawakened print it.
pub fn copy_the_cast_spell(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let spell = item.subject.object().expect("the cast spell");
    g.copy_spell(spell, item.controller, true)?;
    Ok(())
}

/// The delayed ability of "When you next cast an instant or sorcery spell this turn, copy that spell. You may choose new targets
/// for the copy" (CR 603.7b, 707.10), for the card's `delayed` list: set up by `delayed::next_this_turn`, it triggers once on the next
/// instant or sorcery spell its controller casts this turn (the copy is not cast, CR 707.10) and ends with the turn if none is.
/// Way of the Cryomancer and Striking Palette print it.
pub const COPY_THE_NEXT_INSTANT_OR_SORCERY_YOU_CAST: AbilityDef = triggered(
    when_asked(EventMask::SPELL_CAST, crate::triggers::you_cast_instant_or_sorcery),
    copy_the_cast_spell,
    "When you next cast an instant or sorcery spell this turn, copy that spell. You may choose new targets for the copy.",
);
