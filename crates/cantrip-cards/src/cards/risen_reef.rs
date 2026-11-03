use cantrip_api::*;
use cantrip_mechanics::{effects, triggers, top_of_library};
use cantrip_oracle as oracle;

/// "Whenever this creature or another Elemental you control enters, look at the top card of your library. If it's a land card, you may put it onto the battlefield tapped. If you don't put the card onto the battlefield, put it into your hand." (CR 608.2, 400.7).
fn look_top_card_library_if(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let looked = top_of_library(g, item.controller, 1);
    g.look_at(item.controller, &looked);
    g.end_sentence();
    for original in looked {
        let mut entered = false;
        if g.chars(original).is(CardType::Land)
            && effects::ask_may(g, item.controller, item.object, Purpose::Optional)?
        {
            let arrived = g.put_onto_battlefield(original, g.obj(original).owner, true)?;
            entered = g.is_on_battlefield(arrived);
        }
        g.end_sentence();
        if !entered {
            let card = g.found_after_move(original).unwrap_or(original);
            g.return_to_hand(&[card]);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(
                EventMask::ZONE_CHANGE,
                triggers::this_or_another_of_subtype_you_control_enters::<
                    { Subtype::Elemental as u16 },
                >,
            ),
            look_top_card_library_if,
            "Whenever this creature or another Elemental you control enters, look at the top card of your library. If it's a land card, you may put it onto the battlefield tapped. If you don't put the card onto the battlefield, put it into your hand.",
        ),
    ],
    ..oracle::RISEN_REEF
};
