use cantrip_api::*;
use cantrip_mechanics::{devotion::devotion, effects, exile_and_return, exile_and_return_transformed, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

const DISCARD_HAND: Question = Question::new("DiscardHand", "discard your hand, then draw cards equal to your devotion to red?", "", "Discard and draw", "Keep your hand");

/// "When Clive enters, you may discard your hand, then draw cards equal to your devotion to red." (CR 701.9, 700.5, 608.2c): the
/// hand goes first, then devotion is counted - red read through the words of this card's text (CR 612) - as it resolves, Clive
/// himself included if he is still on the battlefield (its ruling). Choosing to discard an empty hand is allowed.
fn discard_hand_then_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::ask_may(g, item.controller, item.object, Purpose::Ask(&DISCARD_HAND))? {
        return Ok(());
    }
    g.discard_hand(item.controller);
    let color = g.words(item.object).color(Color::Red);
    let n = devotion(g, item.controller, color);
    g.draw(item.controller, n);
    Ok(())
}

/// "Exile Clive, then return it to the battlefield transformed under its owner's control." (CR 712.14a): the cost already tapped
/// him; only the object that activated is exiled, and nothing happens if it has left already (CR 400.7).
fn transform(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let owner = g.obj(item.object).owner;
    exile_and_return_transformed(g, item.object, owner)
}

/// "Lunge \u{2014} Ifrit fights up to one other target creature." (CR 701.14): a target that is gone is no fight (CR 701.14b).
fn lunge(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(other) = targets::object(g, item, 0) {
        g.fight(item.object, other);
    }
    Ok(())
}

/// "Brimstone \u{2014} Add {R}{R}{R}{R}. If Ifrit has three or more lore counters on it, exile it, then return it to the battlefield
/// (front face up)." (CR 106.1, 714.2, 712.14): the lore counters are counted as this resolves; if Ifrit has left, he has none
/// and is not the object to exile (CR 400.7). Chapter II has two lore counters, III three.
fn brimstone(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::R, 4);
    if g.is_on_battlefield(item.object) && g.obj(item.object).counters(CounterKind::Lore) >= 3 {
        exile_and_return(g, &[item.object])?;
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        chapter(1, lunge, "I \u{2014} Lunge \u{2014} Ifrit fights up to one other target creature.").targets(&[up_to_one(targets::other_creature_filter)]),
        chapter(2, brimstone, "II \u{2014} Brimstone \u{2014} Add {R}{R}{R}{R}. If Ifrit has three or more lore counters on it, exile it, then return it to the battlefield (front face up)."),
        chapter(3, brimstone, "III \u{2014} Brimstone \u{2014} Add {R}{R}{R}{R}. If Ifrit has three or more lore counters on it, exile it, then return it to the battlefield (front face up)."),
    ],
    ..oracle::IFRIT_WARDEN_OF_INFERNO
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, discard_hand_then_draw, "When Clive enters, you may discard your hand, then draw cards equal to your devotion to red."),
        ability(cost("{4}{R}{R}, {T}"), transform, "{4}{R}{R}, {T}: Exile Clive, then return it to the battlefield transformed under its owner's control. Activate only as a sorcery.").sorcery_speed(),
    ],
    back: Some(&BACK),
    ..oracle::CLIVE_IFRITS_DOMINANT
};
