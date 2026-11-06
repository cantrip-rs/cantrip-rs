use cantrip_api::*;
use cantrip_mechanics::attach::equip;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Whenever equipped creature deals combat damage": a creature the Jitte is attached to dealing more than 0 combat
/// damage. Damage dealt together is one event (CR 603.2c, 510.2), so the ability is `once_per_batch`: however many
/// recipients, it triggers once.
fn equipped_creature_deals_combat_damage(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { source: dealer, amount, kind, .. } if kind.is_combat() && *amount > 0 && q.attached_to(source) == Some(*dealer) => {
            Fired::PLAIN
        }
        _ => None,
    }
}

fn two_charge_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Charge, 2);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(effects::attached_creature_gets::<2, 2>, "Equipped creature gets +2/+2 until end of turn."),
    mode(effects::target_creature_gets::<-1, -1>, "Target creature gets -1/-1 until end of turn.")
        .targets(&[targets::CREATURE]),
    mode(|g, item| cantrip_mechanics::effects::gain_life(g, item, 2), "You gain 2 life."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[
        modal_ability(remove_counters(CounterKind::Charge, 1), &MODAL, "Remove a charge counter from Umezawa's Jitte: Choose one — equipped creature gets +2/+2 until end of turn; target creature gets -1/-1 until end of turn; or you gain 2 life."),
        equip(mana("{2}"), "Equip {2}"),
        triggered(when(EventMask::DAMAGE, equipped_creature_deals_combat_damage), two_charge_counters, "Whenever equipped creature deals combat damage, put two charge counters on Umezawa's Jitte.")
            .once_per_batch(),
    ],
    ..oracle::UMEZAWAS_JITTE
};
