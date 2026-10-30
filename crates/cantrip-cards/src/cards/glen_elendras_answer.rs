use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Counter all spells your opponents control and all abilities your opponents control. Create a 1/1 blue and black Faerie
/// creature token with flying for each spell and ability countered this way." (CR 701.6, 608.2) The items on the stack as it
/// resolves are countered top to bottom (this spell is no longer one of them); a spell that can't be countered stays and
/// is not counted (CR 701.6a). The tokens are made all at once, as many as were countered.
fn counter_and_make_faeries(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let theirs: Vec<u32> = g
        .state()
        .stack
        .iter()
        .rev()
        .filter(|i| g.state().opponents(you).any(|o| o == i.controller))
        .map(|i| i.serial)
        .collect();
    let mut countered = 0;
    for serial in theirs {
        let Some(at) = g.state().stack.iter().position(|i| i.serial == serial) else { continue };
        let target = g.state().stack[at];
        g.counter_item(target.object, at);
        if !g.state().stack.iter().any(|i| i.serial == serial) {
            countered += 1;
        }
    }
    g.end_sentence();
    tokens::create(g, you, &tokens::FAERIE_BLUE_BLACK, countered)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], counter_and_make_faeries), ..oracle::GLEN_ELENDRAS_ANSWER };

pub static EXTRA: &[&CardDef] = &[&tokens::FAERIE_BLUE_BLACK];
