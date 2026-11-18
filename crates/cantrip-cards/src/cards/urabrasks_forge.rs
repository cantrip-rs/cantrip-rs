use cantrip_api::*;
use cantrip_mechanics::{delayed, tokens, triggers};
use cantrip_oracle as oracle;

pub static PHYREXIAN_HORROR: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Creature]),
    colors: Some(ColorSet::of(Color::Red)),
    keywords: Keywords::of(&[Keyword::Trample, Keyword::Haste]),
    ..token!(Phyrexian Horror)
};

/// "At the beginning of combat on your turn, put an oil counter on this artifact, then create an X/1 red Phyrexian Horror creature
/// token with trample and haste, where X is the number of oil counters on this artifact. Sacrifice that token at the beginning
/// of the next end step." (CR 122.1, 111.1, 603.7c). The token is made from this card's text, so a text change reaches its color
/// and creature type (CR 612.2a).
fn put_oil_counter_artifact_create(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Oil, 1);
    g.end_sentence();
    let oil = g.obj(item.object).counters(CounterKind::Oil);
    let created = tokens::create_sized(g, item.object, item.controller, &PHYREXIAN_HORROR, (oil, 1), 1)?;
    g.end_sentence();
    delayed::at_next_end_step_about(g, item, 0, &created);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::BEGINNING_OF_YOUR_COMBAT,
            put_oil_counter_artifact_create,
            "At the beginning of combat on your turn, put an oil counter on this artifact, then create an X/1 red Phyrexian Horror creature token with trample and haste, where X is the number of oil counters on this artifact. Sacrifice that token at the beginning of the next end step.",
        ),
    ],
    delayed: &[delayed_trigger(delayed::sacrifice_created, "Sacrifice that token at the beginning of the next end step.")],
    ..oracle::URABRASKS_FORGE
};

pub static EXTRA: &[&CardDef] = &[&PHYREXIAN_HORROR];
