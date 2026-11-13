use cantrip_api::*;
use cantrip_mechanics::{choose, history};
use cantrip_oracle as oracle;

/// Where the noting ability is in the card's list: "{1}, {T}: ... Note the type of mana spent to pay this activation cost",
/// which the mana ability's "last noted type" is linked to (CR 607.2e).
const NOTES: u8 = 0;

/// "Put a charge counter on this artifact. Note the type of mana spent to pay this activation cost." (CR 122.1, 607.2e): the
/// note is this resolution, which the history keeps (`Event::Resolved`), read back by the mana ability
/// (`history::last_noted_mana`); a countered activation notes nothing.
fn put_charge_counter_and_note(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Charge, 1);
    Ok(())
}

/// "Activate only if there are no charge counters on this artifact" (CR 602.5).
fn no_charge_counters(q: &dyn Query, asker: &Asker) -> bool {
    q.state().obj(asker.source).counters(CounterKind::Charge) == 0
}

/// "Add one mana of this artifact's last noted type." (CR 106.1b, 605.1a): the type, not the amount - one mana of a type
/// spent, its controller choosing among them where more than one type paid the cost (an increased cost); nothing where no
/// type was ever noted, or the note was a copy's, which spent nothing.
fn add_one_mana_of_last_noted_type(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    let noted = history::last_noted_mana(g, g.linked_to(asker), NOTES);
    let kinds: Vec<Mana> = Mana::ALL.into_iter().filter(|&m| noted.get(m) != 0).collect();
    let mana = match kinds.as_slice() {
        [] => return Ok(()),
        [m] => *m,
        _ => kinds[choose::one_of(g, asker.controller, asker.source, kinds.iter().copied().map(Label::ManaType).collect())? as usize],
    };
    g.add_mana(asker.controller, mana, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}").and(TAP),
            put_charge_counter_and_note,
            "{1}, {T}: Put a charge counter on this artifact. Note the type of mana spent to pay this activation cost. Activate only if there are no charge counters on this artifact.",
        )
        .condition(no_charge_counters),
        mana_ability(
            TAP.and(remove_counters(CounterKind::Charge, 1)),
            add_by(add_one_mana_of_last_noted_type),
            "{T}, Remove a charge counter from this artifact: Add one mana of this artifact's last noted type.",
        ),
    ],
    ..oracle::JEWELED_AMULET
};
