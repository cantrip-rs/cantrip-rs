use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Target permanents you own" (CR 108.3): ownership, not control.
fn permanent_you_own(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && q.obj(id).owner == asker.controller)
}

/// Chance: "Discard any number of cards, then draw that many cards."
fn chance(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::discard_any_number_then_draw(g, item.controller, item.object)
}

pub static LEAVE: CardDef = CardDef {
    spell: spell(&[up_to(TargetSpec::ANY_NUMBER, permanent_you_own)], cantrip_mechanics::effects::return_each_target_to_hand),
    ..oracle::LEAVE
};

pub static CHANCE: CardDef = CardDef { spell: spell(&[], chance), ..oracle::CHANCE };

pub static CARD: CardDef = CardDef { split: Some((&LEAVE, &CHANCE)), ..oracle::LEAVE_CHANCE };

pub static EXTRA: &[&CardDef] = &[&LEAVE, &CHANCE];
