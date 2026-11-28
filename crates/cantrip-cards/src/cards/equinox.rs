use std::ops::ControlFlow;

use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, statics::grant_delayed_ability, targets};
use cantrip_oracle as oracle;

/// How many lands `you` controlled the resolution destroyed (CR 701.8a, 701.19c): lands moved from the battlefield by
/// destruction, ones that can't be regenerated included - by the spell itself, since a speculative resolution ends before
/// state-based actions (an animated land dealt lethal damage is destroyed by a rule, not the spell: the rulings). A land
/// sacrificed, or one not destroyed (indestructible, regenerated, a shield counter or an umbra spent instead), is not. Whose
/// it was and that it was a land are read as it last existed on the battlefield (CR 608.2h).
fn lands_destroyed(outcome: &Speculated<'_>, you: PlayerId) -> usize {
    outcome.records.iter().filter(|r| matches!(r.event,
        Event::ZoneChange { old, reason: ZoneReason::Destroy | ZoneReason::DestroyNoRegen, .. }
            if outcome.game.obj(old).controller == you
                && outcome.game.state().last_known(old).is_some_and(|l| l.types.has(CardType::Land)))).count()
}

/// Whether Cleansing ("For each land, destroy that land unless any player pays 1 life") would destroy a land `you` control
/// if it resolved now - the owner's decisions (jschibel, 2026-11-27 and 2026-11-28). Resolved speculatively every way,
/// Cleansing asks every player which lands they pay for before anything is destroyed (CR 101.4, 118.12a, 608.2e), so every
/// combination of answers is its own outcome: both players at 3 life with 13 lands is some 110,000 of them, at 4 life with 17
/// lands some six million, past what the speculation follows (docs/known-issues.md, KI-179).
///
/// What every combination comes to is plain instead. Whether a land is destroyed when no one pays for it is its own: its
/// indestructible (CR 702.12b), its regeneration shield (CR 701.19a), its shield counter (CR 122.1c) or the replacement
/// its controller chooses for it (CR 616.1), none of which a payment for another land changes. So with no life paid
/// Cleansing destroys some number of your lands, the fewest any choice made as they are destroyed can leave - and the
/// players can spare at most as many of those as the life they could pay between them, 1 life a land (CR 119.4): each
/// player's life, or none while they can't pay life (CR 119.8), `payable_resolution_life`, which Cleansing itself asks
/// (Ashiok, Wicked Manipulator's exiling instead lets no one pay more than their life total, its rulings). It would destroy one
/// only if that number is more than that life; otherwise sparing every land it would destroy is a choice the players may
/// make, and a spell that destroys a land only if a choice is made is not countered (the rulings, 2008-10-01).
///
/// The number is read off Cleansing's own resolution, played speculatively along the ways in which no one pays: outcomes
/// are visited depth first, each question's answers in the order offered, and a choice of lands to pay for offers none
/// first (`Query::speculate_resolution`'s contract), so every way with no payment comes first - one, or one for each
/// choice a destruction asks - and the first way with a payment ends the search, after a few runs however many lands
/// and life there are.
fn cleansing_would_destroy_a_land_of(g: &dyn Game, spell: ObjectId, you: PlayerId) -> bool {
    let mut fewest = None;
    speculate(g, Target::Object(spell), |o| {
        if o.records.iter().any(|r| matches!(r.event, Event::ResolutionPaymentBegan { source, .. } if source == spell)) {
            return ControlFlow::Break(());
        }
        // The fewest over the ways no one pays: a choice made as the lands are destroyed may save one (Pyramids or Rest in
        // Peace applied first, CR 616.1).
        fewest = Some(lands_destroyed(o, you).min(fewest.unwrap_or(usize::MAX)));
        ControlFlow::Continue(())
    });
    let payable: u64 = g.state().apnap().map(|player| u64::from(g.payable_resolution_life(player))).sum();
    fewest.is_some_and(|destroyed| destroyed as u64 > payable)
}

/// "{T}: Counter target spell if it would destroy a land you control." (CR 701.6, 608.2): as this resolves, the spell is
/// resolved as it is now on a hidden copy of the game (`speculate_settled`), every way it could go - each answer to each
/// question it asks, each random outcome. It would destroy a land you control only if every way does: one that depends on a
/// choice (Lava Blister, Essence Vortex on a land creature) or on chance is not countered (the rulings). A land destroyed
/// stays destroyed, so a way that has destroyed one already settles the rest of it. "You" is the ability's controller, who
/// activated it (CR 113.8). Cleansing - the printed card, or a copy of it, which has its name (CR 707.10) - is judged by
/// `cleansing_would_destroy_a_land_of` instead.
fn counter_if_it_would_destroy_a_land_you_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = targets::object(g, item, 0) else { return Ok(()) };
    let you = item.controller;
    // Burning of Xinye passes the speculation's bound too; its implementation owes a special case here like Cleansing's
    // (the owner's decision, 2026-11-28: docs/card-notes/burning_of_xinye.md, docs/known-issues.md KI-179).
    let would = if g.chars(spell).name == oracle::CLEANSING.name {
        cleansing_would_destroy_a_land_of(g, spell, you)
    } else {
        speculate_settled(g, Target::Object(spell), |o| lands_destroyed(o, you) > 0,
            |o| if lands_destroyed(o, you) > 0 { ControlFlow::Continue(()) } else { ControlFlow::Break(()) }) == Speculation::Undecided
    };
    if would {
        g.counter(spell);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::LAND),
    delayed: &[ability(TAP, counter_if_it_would_destroy_a_land_you_control, "{T}: Counter target spell if it would destroy a land you control.")
        .targets(&[targets::SPELL])],
    statics: &[static_ability(Layer::Ability, attached_object, grant_delayed_ability,
        "Enchanted land has \"{T}: Counter target spell if it would destroy a land you control.\"")],
    ..oracle::EQUINOX
};
