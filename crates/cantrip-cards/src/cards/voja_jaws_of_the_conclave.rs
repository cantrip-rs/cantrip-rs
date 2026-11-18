use cantrip_api::*;
use cantrip_mechanics::triggers::{ward, THIS_ATTACKS};
use cantrip_mechanics::{effects, permanents_with_subtype_you_control};
use cantrip_oracle as oracle;

/// "Put X +1/+1 counters on each creature you control, where X is the number of Elves you control. Draw a card for each Wolf you
/// control" (CR 122.1, 121.1, 608.2h, 612): each count is worked out as the ability resolves, the Elves and Wolves read through
/// the words of this card's text and Voja herself among the Wolves while she is on the battlefield. The counters go on every creature
/// at once; the Wolves are counted after that sentence, and the cards are drawn together.
fn counters_for_each_elf_then_draw_for_each_wolf(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let elf = g.words(item.object).subtype(Subtype::Elf);
    let x = permanents_with_subtype_you_control(g, item.controller, elf) as i32;
    effects::counters_on_creatures_you_control(g, item.controller, CounterKind::PlusOne, x);
    let wolf = g.words(item.object).subtype(Subtype::Wolf);
    let wolves = permanents_with_subtype_you_control(g, item.controller, wolf);
    g.draw(item.controller, wolves as u32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<3>("Ward {3} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {3}.)"),
        triggered(
            THIS_ATTACKS,
            counters_for_each_elf_then_draw_for_each_wolf,
            "Whenever Voja attacks, put X +1/+1 counters on each creature you control, where X is the number of Elves you control. Draw a card for each Wolf you control.",
        ),
    ],
    ..oracle::VOJA_JAWS_OF_THE_CONCLAVE
};
