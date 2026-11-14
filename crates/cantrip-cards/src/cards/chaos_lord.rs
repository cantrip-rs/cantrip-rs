use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// Count permanents as this resolves, then create a new control effect (CR 603.4, 611.2a).
fn transfer_if_even(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.state().battlefield().len() % 2 == 0 {
        if let Some(player) = item.target(0).player() {
            g.add_effect(item.object, item.object, EffectKind::Control(player), Duration::Indefinite);
        }
    }
    Ok(())
}

/// The attack permission applies only if this object did not enter this turn (CR 302.6).
fn attack_after_entry_turn(q: &dyn Query, asker: &Asker, attacker: ObjectId, _: Target) -> bool {
    attacker == asker.source && !q.history().entered_this_turn(attacker)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            transfer_if_even,
            "At the beginning of your upkeep, target opponent gains control of this creature if the number of permanents is even.",
        ).targets(&[targets::OPPONENT]),
    ],
    attacks_as_though_haste: &[attack_after_entry_turn],
    ..oracle::CHAOS_LORD
};
