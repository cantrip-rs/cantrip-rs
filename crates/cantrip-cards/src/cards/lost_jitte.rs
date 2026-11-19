use cantrip_api::*;
use cantrip_mechanics::attach::equip;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(effects::untap_targets, "Untap target land.").targets(&[targets::LAND]),
    mode(effects::first_target_cant_block_this_turn, "Target creature can't block this turn.").targets(&[targets::CREATURE]),
    mode(effects::plus_one_counter_on_equipped_creature, "Put a +1/+1 counter on equipped creature."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(when(EventMask::DAMAGE, triggers::equipped_creature_deals_combat_damage), effects::counters_on_self::<{ CounterKind::Charge as usize }, 1>, "Whenever equipped creature deals combat damage, put a charge counter on Lost Jitte.")
            .once_per_batch(),
        modal_ability(remove_counters(CounterKind::Charge, 1), &MODAL, "Remove a charge counter from Lost Jitte: Choose one — Untap target land; or target creature can't block this turn; or put a +1/+1 counter on equipped creature."),
        equip(mana("{1}"), "Equip {1}"),
    ],
    ..oracle::LOST_JITTE
};
