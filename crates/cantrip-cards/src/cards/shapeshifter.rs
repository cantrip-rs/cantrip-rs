use cantrip_api::*;
use cantrip_mechanics::{choose, effects, triggers};
use cantrip_oracle as oracle;

/// "As this creature enters, choose a number between 0 and 7. At the beginning of your upkeep, you may choose a number between 0 and 7. Shapeshifter's power is equal to the last chosen number and its toughness is equal to 7 minus that number." (CR 614.1c).
fn as_creature_enters_choose_number_enters(
    g: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    choose::number_as_enters(g, entering, 0, 7, "Choose a number")?;
    Ok(())
}

/// "At the beginning of your upkeep, you may choose a number between 0 and 7." (CR 608.2).
fn choose_number_between_0_7(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::ask_may(g, item.controller, item.object, Purpose::Optional)? {
        let chosen_2: u32 = choose::number(
            g,
            item.controller,
            item.object,
            0,
            7,
            "Choose a number",
        )?;
        let by = {
            let q: &dyn Query = g;
            q.linked(item)
        };
        g.remember_chosen(by, ChosenValue::Number(chosen_2));
        g.end_sentence();
    }
    Ok(())
}

/// "As this creature enters, choose a number between 0 and 7. At the beginning of your upkeep, you may choose a number between 0 and 7. Shapeshifter's power is equal to the last chosen number and its toughness is equal to 7 minus that number." (CR 604.1).
fn as_creature_enters_choose_number_apply_0(
    q: &dyn LayerQuery,
    source: ObjectId,
    chars: &mut Chars,
) {
    let asker = &Asker::new(source, q.controller(source));
    // This text chooses only 0 through 7; an unchosen number is zero (CR 107.2).
    let number = q.state().chosen_number(q.linked_to(asker.source)).unwrap_or(0) as i32;
    chars.power = Some(number);
    chars.toughness = Some(7 - number);
}

pub static CARD: CardDef = CardDef {
    power: Some(0),
    toughness: Some(7),
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            choose_number_between_0_7,
            "At the beginning of your upkeep, you may choose a number between 0 and 7.",
        ),
    ],
    statics: &[
        characteristic_defining(
            Layer::PtDefine,
            as_creature_enters_choose_number_apply_0,
            "Shapeshifter's power is equal to the last chosen number and its toughness is equal to 7 minus that number.",
        ),
    ],
    enters: Some(as_creature_enters_choose_number_enters),
    ..oracle::SHAPESHIFTER
};
