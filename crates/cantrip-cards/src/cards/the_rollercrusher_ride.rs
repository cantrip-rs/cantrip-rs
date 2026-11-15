use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{delirium, replacements, targets};
use cantrip_oracle as oracle;

/// "Up to X target creatures": the X it was cast with (CR 107.3m), read from the enchantment as the ability goes on the stack.
fn x_it_was_cast_with(q: &dyn Query, asker: &Asker) -> u32 {
    u32::from(q.obj(asker.source).x)
}

/// "It deals X damage to each of up to X target creatures": all at once (CR 120.2), by the enchantment, which is still the source
/// when it has left the battlefield (last-known information, CR 608.2h). The X is read from it as it resolves, as it is when the
/// ability goes on the stack, and is 0 when it was put onto the battlefield without being cast.
fn deal_x_to_each(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = u32::from(g.obj(item.object).x);
    let hits: Vec<_> = targets::legal_objects(g, item).into_iter().map(|creature| (item.object, Target::Object(creature), x)).collect();
    g.damage_together(&hits);
    Ok(())
}

/// "If a source you control would deal noncombat damage to a permanent or player while there are four or more card types among
/// cards in your graveyard" (CR 614.1a, 120.3, 207.2c): damage of any source its controller controls (a spell or an ability on the
/// stack, a permanent), of an amount above zero (CR 615.6), that is not combat damage, dealt to anything; the graveyard is counted
/// when the damage is about to be dealt.
fn noncombat_damage_with_delirium(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    delirium(q, asker.controller) && matches!(proposal, Proposal::Damage(d) if !d.combat && d.amount > 0 && q.controller(d.source) == asker.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, deal_x_to_each, "When The Rollercrusher Ride enters, it deals X damage to each of up to X target creatures.")
        .targets(&[targets::CREATURE.between(0, TargetSpec::COUNTED)])
        .counted_by(x_it_was_cast_with)],
    replacements: &[replacement(
        ReplacementKind::Damage,
        noncombat_damage_with_delirium,
        replacements::double_that_damage,
        "Delirium — If a source you control would deal noncombat damage to a permanent or player while there are four or more card types among cards in your graveyard, it deals double that damage instead.",
    )],
    ..oracle::THE_ROLLERCRUSHER_RIDE
};
