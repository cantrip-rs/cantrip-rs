//! Linked noted mana: Ice Cauldron and Jeweled Amulet (CR 607.2e).
use cantrip_api::*;

/// The most recently noted value of this kind, from exactly this linked text.
pub fn last_mana(q: &dyn Query, by: Linked) -> ManaPool {
    q.history()
        .all()
        .iter()
        .rev()
        .find_map(|r| {
            let Event::ValueNoted { value } = r.event else {
                return None;
            };
            let record = q.state().noted_values.get(value as usize)?;
            match &record.value {
                NotedValue::Mana(mana) if record.by == by => Some(*mana),
                _ => None,
            }
        })
        .unwrap_or_default()
}

/// Record this activation's payment, not a later activation's or a nested mana payment's.
/// A copy was not activated and spent no mana on its own activation cost (CR 707.10).
/// Type-only notes discard amounts without discarding the colorless type.
pub fn note_mana_spent(g: &mut dyn Game, item: &StackItem, amount: bool) {
    let by = g.linked(item);
    let mana = g
        .history()
        .all()
        .iter()
        .rev()
        .find_map(|r| {
            let Event::ActivationManaPaid { serial, value } = r.event else {
                return None;
            };
            if item.copied || serial != item.serial {
                return None;
            }
            let record = g.state().noted_values.get(value as usize)?;
            match &record.value {
                NotedValue::Mana(mana) if record.by == by => Some(*mana),
                _ => None,
            }
        })
        .unwrap_or_default();
    let mana = if amount {
        mana
    } else {
        ManaPool(mana.0.map(|n| u32::from(n != 0)))
    };
    g.note_value(by, NotedValue::Mana(mana));
}

/// Add the noted amount and kinds, or one mana of a noted kind. Restrictions belong to this production.
/// Empty/never-recorded notes add nothing; unrelated charge counters do not create information.
pub fn add_last_mana(
    g: &mut dyn Game,
    asker: &Asker,
    player: PlayerId,
    amount: bool,
    only: Option<ManaRestriction>,
) -> Result<(), Suspend> {
    let by = g.linked_to(asker);
    let mut mana = last_mana(g, by);
    if !amount {
        let kinds: Vec<_> = Mana::ALL
            .into_iter()
            .filter(|&m| mana.get(m) != 0)
            .collect();
        let chosen = match kinds.as_slice() {
            [] => return Ok(()),
            [m] => *m,
            _ => {
                kinds[crate::choose::one_of(
                    g,
                    player,
                    asker.source,
                    kinds.iter().copied().map(Label::ManaType).collect(),
                )? as usize]
            }
        };
        mana = ManaPool::default();
        mana.0[chosen as usize] = 1;
    }
    for kind in Mana::ALL {
        let n = mana.get(kind);
        if n == 0 {
            continue;
        }
        match only {
            Some(only) => g.add_restricted_mana(player, kind, n, only),
            None => g.add_mana(player, kind, n),
        }
    }
    Ok(())
}
