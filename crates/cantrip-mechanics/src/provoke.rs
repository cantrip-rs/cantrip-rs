//! Provoke (CR 702.39a), a triggered ability: "Whenever this creature attacks, you may choose to have target creature
//! defending player controls block this creature this combat if able. If you do, untap that creature." Each instance
//! triggers separately (CR 702.39b). Brontotherium, Crested Craghorn, Goblin Grappler and Lowland Tracker print it.
//!
//! The block is a requirement (CR 509.1c): `EffectKind::MustBlockAttacker`, which lasts this combat and is obeyed only as far
//! as the restrictions allow - a creature that can't block this one, or is tapped by something that taps it again, blocks as it
//! may. Untapping is part of what the choice does, so it happens whether or not the creature ends up able to block.
//!
//! A card with provoke lists `TRIGGER` among its `abilities`.

use crate::effects::ask_may;
use crate::targets;
use crate::triggers::this_attacks;
use cantrip_api::*;

/// "Target creature defending player controls" (CR 115.1, 508.5): a creature controlled by the player the ability's source
/// attacks - or, for a source that attacks a planeswalker or battle, that permanent's controller or protector.
fn creature_defending_player_controls(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::on_battlefield(q, id) && q.chars(id).is(CardType::Creature) && q.defending_player(asker.source) == Some(q.controller(id)))
}

const ASK: Question = Question::new(
    "Provoke",
    "have the creature untap and block this creature if able?",
    "Provoke: it blocks this creature this combat if able",
    "Provoke it",
    "Don't",
);

/// The choice is made as the ability resolves (CR 702.39a "you may choose"): the creature untaps and must block the attacker
/// this combat if able. Its target illegal, the ability does not resolve (CR 608.2b), so there is always a creature here.
fn provoke(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    if ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        g.untap(creature);
        g.add_effect(item.object, creature, EffectKind::MustBlockAttacker(item.object), Duration::EndOfCombat);
    }
    Ok(())
}

pub const TRIGGER: AbilityDef = triggered(
    when(EventMask::ATTACKS, this_attacks),
    provoke,
    "Provoke (Whenever this creature attacks, you may have target creature defending player controls untap and block it if able.)",
)
.targets(&[target(creature_defending_player_controls)]);
