use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// The land does not untap during its controller's untap step while it has a depletion counter (CR 502.3).
fn depletion_prevents_untap(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    id == asker.source && q.obj(id).counters(CounterKind::Depletion) > 0
}

fn remove_depletion(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.remove_counters(item.object, CounterKind::Depletion, 1);
    Ok(())
}

fn add_mana_and_depletion(g: &mut dyn Game, asker: &Asker, chosen: Option<Color>) -> Result<(), Suspend> {
    let Some(color) = chosen else { return Ok(()) };
    g.add_mana(asker.controller, color.mana(), 1);
    g.end_sentence();
    g.add_counters(asker.source, CounterKind::Depletion, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::YOUR_UPKEEP, remove_depletion, "At the beginning of your upkeep, remove a depletion counter from this land."),
        mana_ability(TAP, add_by(add_mana_and_depletion).choose_color()
            .colors(ColorSet(ColorSet::of(Color::White).0 | ColorSet::of(Color::Blue).0)).yields_any(1),
            "{T}: Add {W} or {U}. Put a depletion counter on this land."),
    ],
    untap_bans: &[depletion_prevents_untap],
    ..oracle::LAND_CAP
};
