use cantrip_api::*;
use cantrip_mechanics::{half_of_library_rounded_up, targets, top_of_library};
use cantrip_mechanics::triggers::{ANNIHILATOR, CAST_THIS_SPELL, annihilate_n, ward_sacrifice};
use cantrip_oracle as oracle;

/// "When you cast this spell, target opponent exiles the top half of their library, rounded up." It resolves before
/// Ulamog does, whatever becomes of the spell (the ruling); the half is of the library as the ability resolves.
fn exile_half(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = item.target(0).player() else { return Ok(()) };
    let half = half_of_library_rounded_up(g, opponent) as usize;
    let top = top_of_library(g, opponent, half);
    g.exile_all(&top);
    Ok(())
}

/// "Ulamog enters with a number of +1/+1 counters on it equal to the greatest mana value among cards in exile" (CR 614.1c):
/// every player's exile, as it enters. Entering from exile it is still there and sees itself (the ruling).
fn enters(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let greatest = g.state().seats().flat_map(|p| g.state().zone(Zone::exile(p)).iter().copied()).map(|c| g.mana_value(c)).max().unwrap_or(0);
    entering.counters[CounterKind::PlusOne as usize] += greatest.min(i16::MAX as u32) as i16;
    Ok(())
}

/// "Ulamog has annihilator X, where X is the number of +1/+1 counters on it" (CR 702.86a): X is counted as the ability
/// resolves (the ruling), on Ulamog as it is then or as it last was on the battlefield.
fn annihilate_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = g.obj(item.object).counters(CounterKind::PlusOne).clamp(0, u8::MAX as i16) as u8;
    annihilate_n(g, item, x)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(CAST_THIS_SPELL, exile_half, "When you cast this spell, target opponent exiles the top half of their library, rounded up.")
            .targets(&[targets::OPPONENT]),
        ward_sacrifice::<2>("Ward\u{2014}Sacrifice two permanents."),
        triggered(ANNIHILATOR, annihilate_x, "Ulamog has annihilator X, where X is the number of +1/+1 counters on it."),
    ],
    enters: Some(enters),
    ..oracle::ULAMOG_THE_DEFILER
};
