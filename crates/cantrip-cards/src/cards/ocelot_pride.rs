use cantrip_api::*;
use cantrip_mechanics::history::{entered_this_turn, you_gained_life_this_turn};
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// The 1/1 white Cat creature token it makes.
pub static CAT: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Cat)
};

/// Create a 1/1 white Cat creature token. Then, if you have the city's blessing, for each token you control that entered this
/// turn, create a token that's a copy of it: the board is read once, after the Cat has arrived, so the Cat is copied and the
/// copies are not; every copy enters at once (CR 101.4).
fn cats(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    tokens::create(g, you, &CAT, 1)?;
    if !g.state().player(you).citys_blessing {
        return Ok(());
    }
    let tokens: Vec<ObjectId> =
        g.permanents_of(you).into_iter().filter(|&t| g.obj(t).token && entered_this_turn(g.history(), t)).collect();
    if tokens.iter().all(|&t| g.obj(t).copy.is_none()) {
        let copies: Vec<(CardId, PlayerId)> = tokens.iter().map(|&t| (g.obj(t).card, you)).collect();
        // A copy has the copiable values of what it copies (CR 707.2), not the words of Ocelot Pride's text.
        g.create_tokens_worded(&copies, false, None, Words::NONE)?;
    } else {
        // Copies of tokens that are themselves copies keep their exceptions (CR 707.2); they too enter at once (CR 101.4).
        let copies: Vec<(Copiable, PlayerId)> = tokens.iter().map(|&t| (g.copiable_values(t), you)).collect();
        g.create_token_copies_together(&copies)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    ascend: true,
    abilities: &[triggered(triggers::YOUR_END_STEP, cats, "At the beginning of your end step, if you gained life this turn, create a 1/1 white Cat creature token. Then if you have the city's blessing, for each token you control that entered this turn, create a token that's a copy of it.")
        .condition(you_gained_life_this_turn)],
    ..oracle::OCELOT_PRIDE
};

pub static EXTRA: &[&CardDef] = &[&CAT];
