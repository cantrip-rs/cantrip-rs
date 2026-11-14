use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Put three +1/+1 counters on this creature and it becomes a black Demon in addition to its other colors and types" (CR 122.1,
/// 105.3, 205.1b, 613.1d-e): the counters go on first (they stay with the object whatever happens next); then, if this creature is
/// still on the battlefield (a new object after a zone change, CR 400.7, has a new id), it becomes black as well and a Demon as well, for as long as it is on
/// the battlefield (`Duration::Indefinite`). The type is read through the words of the text (CR 612). The discard is a cost
/// (CR 602.2) and "activate only once" (CR 602.5b) is `once`.
fn possess(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::three_plus_one_counters_on_this(g, item)?;
    g.end_sentence();
    let goat = item.object;
    if !g.is_on_battlefield(goat) {
        return Ok(());
    }
    let demon = g.words(goat).subtype(Subtype::Demon);
    instruction(g, |g| {
        g.add_effect(goat, goat, EffectKind::AddColors(ColorSet::of(Color::Black)), Duration::Indefinite);
        g.add_effect(goat, goat, EffectKind::AddSubtype(demon), Duration::Indefinite);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}").and(discard_cards(1)),
        possess,
        "{3}, Discard a card: Put three +1/+1 counters on this creature and it becomes a black Demon in addition to its other colors and types. Activate only once.",
    )
    .once()],
    ..oracle::POSSESSED_GOAT
};
