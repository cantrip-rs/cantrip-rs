use cantrip_api::*;
use cantrip_mechanics::collect_evidence::may_collect_evidence;
use cantrip_mechanics::targets::token_you_control;
use cantrip_mechanics::triggers::THIS_ENTERS_OR_ATTACKS;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// The 2/1 black and green Spider creature token with reach and menace.
static SPIDER: CardDef = CardDef {
    colors: Some(ColorSet(ColorSet::of(Color::Black).0 | ColorSet::of(Color::Green).0)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Reach, Keyword::Menace]),
    ..token!(Spider)
};

/// "You may collect evidence 4. If you do, create two 2/1 black and green Spider creature tokens with reach and menace." (CR 701.59a,
/// 608.2d, 111.1): the evidence is exiled from the controller's graveyard as the ability resolves, and the Spiders are made only if it was.
fn evidence_for_spiders(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if may_collect_evidence::<4>(g, item)? {
        tokens::create(g, item.controller, &SPIDER, 2)?;
    }
    Ok(())
}

/// "Surveil 2, then draw two cards. You gain 2 life." (CR 701.25, 121.1, 119.3): the surveil is complete before the draw, the life is
/// gained as the last sentence.
fn surveil_draw_gain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 2)?;
    effects::draw(g, item, 2)?;
    g.gain_life(item.controller, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS_OR_ATTACKS,
            evidence_for_spiders,
            "Whenever Izoni enters or attacks, you may collect evidence 4. If you do, create two 2/1 black and green Spider creature tokens with reach and menace.",
        ),
        ability(sacrifice(4, token_you_control), surveil_draw_gain, "Sacrifice four tokens: Surveil 2, then draw two cards. You gain 2 life."),
    ],
    ..oracle::IZONI_CENTER_OF_THE_WEB
};

pub static EXTRA: &[&CardDef] = &[&SPIDER];
