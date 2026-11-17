use cantrip_api::*;
use cantrip_mechanics::plot::plot;
use cantrip_oracle as oracle;

/// "This creature enters with two +1/+1 counters on it if it wasn't cast or no mana was spent to cast it" (CR 614.1c, 122.6):
/// the `enters` replacement. Nothing was spent (CR 601.2h) on a creature put onto the battlefield, or cast without paying its
/// mana cost (plotted, CR 702.170d), so one total of zero answers both halves; one cast with any mana, whatever it paid for,
/// enters without counters. Plot (CR 702.170) is the shared special action.
fn enters_with_two_counters_if_unpaid(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if g.obj(entering.object).spent.total() == 0 {
        entering.counters[CounterKind::PlusOne as usize] += 2;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_with_two_counters_if_unpaid),
    abilities: &[plot(
        mana("{3}{G}"),
        "Plot {3}{G} (You may pay {3}{G} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
    )],
    ..oracle::FREESTRIDER_COMMANDO
};
