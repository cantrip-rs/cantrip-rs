use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Only this creature's owner may activate this ability." (CR 602.2): its owner, whoever controls it - and not its
/// controller, where that is another player.
fn owner_may_activate(q: &dyn Query, asker: &Asker, player: PlayerId) -> bool {
    q.obj(asker.source).owner == player
}

/// "The next 1 damage that would be dealt to this creature this turn" (CR 614.1a): damage to the creature the effect is
/// about, while it is that object (CR 400.7). The core asks no effect about damage prevented to nothing, which is no damage
/// (CR 615.1), so such damage does not use the effect up (CR 609.7b).
fn next_damage_to_it(_: &dyn Query, _: &Asker, creature: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Damage(d) if d.target == Target::Object(creature))
}

/// "... is dealt to its owner instead" (CR 614.9): 1 of that damage is no longer dealt to the creature; the rest still is.
fn one_less(_: &dyn Query, _: &Asker, _: ObjectId, proposal: &mut Proposal) {
    if let Proposal::Damage(d) = proposal {
        d.amount -= 1;
    }
}

/// The 1 damage taken from the creature is dealt to its owner by the same source, in the same damage event (CR 614.6,
/// 614.9, 120.4): one damage event with the creature's share, combat damage still combat damage. Each activation makes
/// its own effect, each redirecting 1, applied one after another to what the others left (CR 616.1f).
fn to_its_owner(q: &dyn Query, _: &Asker, creature: ObjectId, before: &DamageEvent, after: &DamageEvent) -> Vec<DamageComponent> {
    vec![DamageComponent::Redirected { target: Target::Player(q.obj(creature).owner), amount: before.amount - after.amount }]
}

/// The effect is made as the ability resolves (CR 611.2), about this creature, and lasts until it is used up or the turn
/// ends.
fn redirect_next_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let kind = EffectKind::Replacement { card: item.ability.card, index: 0, to: Target::None, into: Target::None };
    g.add_effect(item.object, item.object, kind, Duration::EndOfTurn);
    Ok(())
}

/// "When this creature dies, its owner loses half their life, rounded up." (CR 700.4, 119.3, 107.1a): its owner, not its
/// last controller (the ruling); a life total of zero or less loses none (the ruling).
fn owner_loses_half(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let owner = g.obj(item.object).owner;
    effects::lose_half_their_life(g, owner);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            Cost::mana(ManaCost::ZERO),
            redirect_next_one,
            "{0}: The next 1 damage that would be dealt to this creature this turn is dealt to its owner instead. Only this creature's owner may activate this ability.",
        )
        .activated_by(owner_may_activate),
        triggered(triggers::THIS_DIES, owner_loses_half, "When this creature dies, its owner loses half their life, rounded up."),
    ],
    granted_replacements: &[granted_replacement(ReplacementKind::Damage, next_damage_to_it, one_less, "1 of the damage is dealt to its owner instead")
        .once()
        .with_damage(to_its_owner)
        .text("The next 1 damage that would be dealt to this creature this turn is dealt to its owner instead.")],
    ..oracle::PERSONAL_INCARNATION
};
