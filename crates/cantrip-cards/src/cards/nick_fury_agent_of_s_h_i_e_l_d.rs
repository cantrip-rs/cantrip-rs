use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::{library, power_up};
use cantrip_oracle as oracle;

const TRANSFORM: Question = Question::new(
    "Transform",
    "transform it?",
    "The double-faced card Nick Fury put onto the battlefield turns over to its other face",
    "Transform",
    "Don't",
);

/// "Put two +1/+1 counters on Nick Fury, then look at the top seven cards of your library. You may put a Hero, Equipment, or Vehicle
/// card from among them onto the battlefield. If it's a double-faced card, you may transform it. Put the rest on the bottom of your
/// library in a random order." (CR 122.1, 401.2, 701.27a, 401.4, 608.2c.) The counters are his, if he is still on the battlefield
/// (CR 400.7); the seven cards are looked at whatever became of him. Hero is a creature type of this card's text (CR 612), Equipment
/// and Vehicle artifact types. The card put onto the battlefield is offered the transformation before the rest are placed.
fn power_up_effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 2);
    // "Then": a new sentence (CR 608.2c).
    g.end_sentence();
    let hero = g.words(item.object).subtype(Subtype::Hero);
    let wanted = move |q: &dyn Query, card: ObjectId| {
        let chars = q.chars(card);
        chars.has_subtype(hero) || chars.has_subtype(Subtype::Equipment) || chars.has_subtype(Subtype::Vehicle)
    };
    library::look_at_top_onto_battlefield_then(
        g,
        item.controller,
        item.object,
        7,
        1,
        "Hero, Equipment, or Vehicle",
        wanted,
        None,
        false,
        library::Rest::BottomRandom,
        |g, arrived| {
            g.end_sentence();
            for &permanent in arrived {
                let double_faced = g.obj(permanent).represented_double_faced(g.card(g.obj(permanent).card).back.is_some());
                if double_faced && ask_may(g, item.controller, item.object, Purpose::Ask(&TRANSFORM))? {
                    g.transform(permanent)?;
                }
            }
            Ok(())
        },
    )?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[power_up::ability(
        cost("{W}{U}{B}{R}{G}"),
        power_up_effect,
        "Power-up \u{2014} {W}{U}{B}{R}{G}: Put two +1/+1 counters on Nick Fury, then look at the top seven cards of your library. You may put a Hero, Equipment, or Vehicle card from among them onto the battlefield. If it's a double-faced card, you may transform it. Put the rest on the bottom of your library in a random order. (Activate each power-up ability only once. Reduce the cost by his mana cost if he entered this turn.)",
    )],
    ..oracle::NICK_FURY_AGENT_OF_S_H_I_E_L_D
};
