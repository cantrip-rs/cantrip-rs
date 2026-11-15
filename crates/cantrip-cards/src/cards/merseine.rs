use cantrip_api::*;
use cantrip_mechanics::{attach, effects, targets};
use cantrip_oracle as oracle;

/// The mana cost of the creature `aura` enchants, or none when that creature has no mana cost (CR 202.1b): a token or a card
/// printed with none, a copy of one (CR 707.9b), a face-down permanent (CR 708.2a), a back face, or a Room with no door
/// unlocked (CR 709.5). `Query::mana_cost` gives each of those as {0}, which is a cost that can be paid; theirs can't.
fn enchanted_mana_cost(q: &dyn Query, aura: ObjectId) -> Option<ManaCost> {
    let creature = q.obj(aura).attached_to?;
    let chars = q.chars(creature);
    let def = q.card(chars.card);
    let none = chars.no_cost || !def.has_cost || (def.is_room() && q.obj(creature).marks.doors == 0);
    (!none).then(|| q.mana_cost(creature))
}

/// "Enchanted creature doesn't untap during its controller's untap step if this Aura has a net counter on it" (CR 502.3):
/// asked of the Aura's own counters as the untap step comes, not the creature's.
fn netted(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    attach::attached_to_it(q, asker, id) && q.obj(asker.source).counters(CounterKind::Net) > 0
}

/// "Pay enchanted creature's mana cost" (CR 602.2b, 601.2f): the activation cost is the mana cost the enchanted creature has
/// as the total cost is determined, its colored, hybrid and Phyrexian symbols as they are (the ruling: "the cost that needs to
/// be paid includes colored mana requirements"), and any {X} in it 0 (CR 107.3h), so no {X} symbol is left in it. The
/// printed cost holds no other mana, so the whole of it comes in here, before any reduction applies (CR 601.2f); how its
/// Phyrexian and {2/C} symbols are paid is announced as the ability is proposed (CR 118.13a), as the core does for an own
/// change.
fn enchanted_creatures_mana_cost(q: &dyn Query, asker: &Asker, _: Costed, _: &[Target]) -> CostDelta {
    enchanted_mana_cost(q, asker.source).map_or(CostDelta::NONE, |cost| CostDelta::more_mana(ManaCost { x: 0, ..cost }))
}

/// A creature with no mana cost makes the ability's cost unpayable (CR 118.6): it can't be activated, since the cost could
/// never be paid.
fn payable(q: &dyn Query, asker: &Asker) -> bool {
    enchanted_mana_cost(q, asker.source).is_some()
}

/// "Only the controller of the enchanted creature may activate this ability" (CR 602.2): that player, whoever controls the
/// Aura, and no one else - the Aura's controller included, unless they control the creature too.
fn enchanted_creatures_controller(q: &dyn Query, asker: &Asker, player: PlayerId) -> bool {
    q.obj(asker.source).attached_to.is_some_and(|creature| q.controller(creature) == player)
}

pub static CARD: CardDef = CardDef {
    // "This Aura enters with three net counters on it" (CR 122.6, 614.1c).
    enters: Some(effects::enters_with_counters::<{ CounterKind::Net as usize }, 3>),
    abilities: &[ability(
        Cost::NONE,
        effects::remove_a_counter_from_this::<{ CounterKind::Net as usize }>,
        "Pay enchanted creature's mana cost: Remove a net counter from this Aura. Only the controller of the enchanted creature may activate this ability.",
    )
    .cost_change(enchanted_creatures_mana_cost)
    .condition(payable)
    .activated_by(enchanted_creatures_controller)],
    untap_bans: &[netted],
    enchant: Some(targets::CREATURE),
    ..oracle::MERSEINE
};
