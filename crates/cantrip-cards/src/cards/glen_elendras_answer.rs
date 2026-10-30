use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Counter all spells your opponents control and all abilities your opponents control. Create a 1/1 blue and black Faerie
/// creature token with flying for each spell and ability countered this way." (CR 701.6, 608.2) One instruction counters them
/// all at once (CR 608.2c), so the spells reach their owners' graveyards together and a "one or more" ability triggers once
/// (CR 603.2c); this spell is no longer on the stack. A spell that can't be countered stays and is not counted (CR 101.2,
/// 701.6a). The tokens are made all at once, as many as were countered.
fn counter_and_make_faeries(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let theirs: Vec<(ObjectId, usize)> = g
        .state()
        .stack
        .iter()
        .enumerate()
        .filter(|(_, i)| g.state().opponents(you).any(|o| o == i.controller))
        .map(|(at, i)| (i.object, at))
        .collect();
    let countered = g.counter_items(&theirs);
    g.end_sentence();
    tokens::create(g, you, &tokens::FAERIE_BLUE_BLACK, countered)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], counter_and_make_faeries), ..oracle::GLEN_ELENDRAS_ANSWER };

pub static EXTRA: &[&CardDef] = &[&tokens::FAERIE_BLUE_BLACK];
