use cantrip_api::*;
use cantrip_mechanics::plot::plot;
use cantrip_oracle as oracle;

/// "If you control five or more untapped lands, this creature enters with two +1/+1 counters and a lifelink counter on it"
/// (CR 614.1c, 122.1b, 122.6): the untapped lands the player it enters under controls as it enters; it is not on the battlefield
/// yet, and the opponents' lands are not counted.
fn enters_with_counters_given_five_untapped_lands(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let untapped = g.permanents_of(entering.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Land) && !g.obj(id).tapped).count();
    if untapped >= 5 {
        entering.counters[CounterKind::PlusOne as usize] += 2;
        entering.counters[CounterKind::Lifelink as usize] += 1;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[plot(
        mana("{1}{W}"),
        "Plot {1}{W} (You may pay {1}{W} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
    )],
    enters: Some(enters_with_counters_given_five_untapped_lands),
    ..oracle::DUST_ANIMUS
};
