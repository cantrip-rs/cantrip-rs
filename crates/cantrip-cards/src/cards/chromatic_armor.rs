use cantrip_api::*;
use cantrip_mechanics::{choose_color, effects, targets};
use cantrip_oracle as oracle;

/// "As this Aura enters, choose a color. This Aura enters with a sleight counter on it. Prevent all damage that would be dealt to enchanted creature by sources of the last chosen color. {X}: Put a sleight counter on this Aura and choose a color. X is the number of sleight counters on this Aura." (CR 614.1c).
fn put_sleight_counter_aura_choose_enters(
    g: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    entering.chosen_color = Some(choose_color(g, entering.controller, entering.object)?);
    Ok(())
}

/// "As this Aura enters, choose a color. This Aura enters with a sleight counter on it. Prevent all damage that would be dealt to enchanted creature by sources of the last chosen color. {X}: Put a sleight counter on this Aura and choose a color. X is the number of sleight counters on this Aura." (CR 615.1).
fn put_sleight_counter_aura_choose_applies(
    q: &dyn Query,
    asker: &Asker,
    e: &DamageEvent,
) -> bool {
    {
        let (source, target) = (e.source, &e.target);
        if q
            .state()
            .chosen_color(q.linked_to(asker))
            .is_some_and(|chosen| q.chars(source).colors.has(chosen))
            && matches!(
                * target, Target::Object(o) if Some(asker.source).and_then(| o : ObjectId
                | q.attached_to(o)) == Some(o)
            )
        {
            return true;
        }
    }
    false
}

/// "{X}: Put a sleight counter on this Aura and choose a color. X is the number of sleight counters on this Aura." (CR 608.2, 122.1).
fn put_sleight_counter_aura_choose(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    g.add_counters_together(&[(item.object, CounterKind::Sleight, 1)]);
    let color = choose_color(g, item.controller, item.object)?;
    let by = g.linked(item);
    g.remember_chosen(by, ChosenValue::Color(color));
    Ok(())
}

/// "{X}: Put a sleight counter on this Aura and choose a color. X is the number of sleight counters on this Aura." (CR 608.2).
fn put_sleight_counter_aura_choose_x_is(
    q: &dyn Query,
    asker: &Asker,
    _: &[Target],
) -> u32 {
    q.state().obj(asker.source).counters(CounterKind::Sleight) as u32
}

/// "As this Aura enters, choose a color. This Aura enters with a sleight counter on it. Prevent all damage that would be dealt to enchanted creature by sources of the last chosen color. {X}: Put a sleight counter on this Aura and choose a color. X is the number of sleight counters on this Aura." (CR 614.1c).
fn entering_all(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    put_sleight_counter_aura_choose_enters(g, entering)?;
    effects::enters_with_a_counter::<{ CounterKind::Sleight as usize }>(g, entering)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{X}"),
                put_sleight_counter_aura_choose,
                "{X}: Put a sleight counter on this Aura and choose a color. X is the number of sleight counters on this Aura.",
            )
            .x_is(put_sleight_counter_aura_choose_x_is),
    ],
    preventions: &[
        PreventionDef {
            hint: "prevents all damage that would be dealt to enchanted creature by sources of the last chosen color",
            applies: put_sleight_counter_aura_choose_applies,
            amount: PreventionAmount::All,
        },
    ],
    enters: Some(entering_all),
    enchant: Some(targets::CREATURE),
    ..oracle::CHROMATIC_ARMOR
};
