use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// Discard your hand (together, nothing chosen), then investigate four times.
fn discard_and_investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.discard_hand(item.controller);
    for _ in 0..4 {
        tokens::investigate(g, item.controller)?;
    }
    Ok(())
}

/// "As long as you have one or fewer cards in hand": only this creature, while its controller's hand allows.
fn affects(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    let state = q.state();
    source == affected && state.zone(Zone::hand(state.obj(source).controller)).len() <= 1
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, discard_and_investigate, "When this creature enters, discard your hand, then investigate four times.")],
    statics: &[
        static_ability(Layer::PtModify, affects, cantrip_mechanics::statics::plus_pt::<2, 2>, "As long as you have one or fewer cards in hand, this creature gets +2/+2."),
        static_ability(Layer::Ability, affects, cantrip_mechanics::statics::have_menace, "As long as you have one or fewer cards in hand, this creature has menace."),
    ],
    ..oracle::CARNAGE_INTERPRETER
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
