use cantrip_api::*;
use cantrip_mechanics::{replacements, triggers};
use cantrip_oracle as oracle;

/// "Stagger — Whenever Lightning deals combat damage to a player, until your next turn, if a source would deal damage to that
/// player or a permanent that player controls, it deals double that damage instead" (CR 614.1, 608.2h, 611.2a): the
/// replacement rule is made for the player the trigger is about (its subject), kept as it is made, so a later Lightning
/// trigger for another player makes a second, separate rule (CR 616.1). It lasts until the start of the controller's next turn.
fn stagger(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_player_effect(
        item.controller,
        item.object,
        EffectKind::ReplacementRule { card: item.ability.card, index: 0, value: 0, player: item.subject.player() },
        Duration::UntilNextTurnOf(item.controller),
    );
    Ok(())
}

/// "... if a source would deal damage to that player or a permanent that player controls" (CR 614.1, 120.3): damage of any
/// source, combat or not, dealt to the player this effect names or to a permanent they control. Damage already prevented to
/// nothing is no longer proposed, so a fully prevented hit has nothing to double (the ruling).
fn damage_to_that_player_or_their_permanent(q: &dyn Query, _: &Asker, _: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Damage(d) if q.replacement_player().is_some_and(|named| {
        d.target.player() == Some(named) || d.target.object().is_some_and(|permanent| q.controller(permanent) == named)
    }))
}

/// "... it deals double that damage instead" is `replacements::double_that_damage`: twice what the effects applied before it
/// have left (CR 616.1f), from the same source (CR 614.5).
fn double(q: &dyn Query, asker: &Asker, _: ObjectId, proposal: &mut Proposal) {
    replacements::double_that_damage(q, asker, proposal);
}

const STAGGER: &str = "Stagger \u{2014} Whenever Lightning deals combat damage to a player, until your next turn, if a source would deal damage to that player or a permanent that player controls, it deals double that damage instead.";

/// First strike, trample and lifelink are core keywords.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when(EventMask::DAMAGE, triggers::deals_combat_damage_to_player_subject), stagger, STAGGER)],
    granted_replacements: &[granted_replacement(ReplacementKind::Damage, damage_to_that_player_or_their_permanent, double, "deals double that damage instead").text(STAGGER)],
    ..oracle::LIGHTNING_ARMY_OF_ONE
};
