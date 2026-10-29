use cantrip_api::*;
use cantrip_mechanics::{effects, prepare, triggers};
use cantrip_oracle as oracle;

/// "You draw a card and gain 1 life." (CR 121.1, 119.1): one sentence, so the draw and the life gain are done in turn.
fn draw_and_gain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)?;
    g.end_sentence();
    effects::gain_life(g, item, 1)
}

/// Tam's prepare spell (CR 722): not a card here, only ever a copy.
pub static DEEP_SIGHT: CardDef = CardDef {
    name: "Deep Sight",
    cost: mana("{G}{U}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], draw_and_gain),
    text: "You draw a card and gain 1 life.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    prepare: Some(&DEEP_SIGHT),
    abilities: &[triggered(
        triggers::LANDFALL,
        prepare::become_prepared,
        "Landfall \u{2014} Whenever a land you control enters, Tam becomes prepared.",
    )],
    ..oracle::TAM_OBSERVANT_SEQUENCER
};

pub static EXTRA: &[&CardDef] = &[&DEEP_SIGHT];
