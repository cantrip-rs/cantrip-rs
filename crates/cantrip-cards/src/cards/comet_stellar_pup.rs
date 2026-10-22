use cantrip_api::*;
use cantrip_mechanics::{choose, haste_until_end_of_turn};
use cantrip_oracle as oracle;

/// A 1/1 green Squirrel creature token.
pub static SQUIRREL: CardDef = CardDef {
    name: "Squirrel Token",
    token_name: Some(&[Subtype::Squirrel]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Squirrel]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

fn loyalty(g: &mut dyn Game, comet: ObjectId, delta: i16) {
    if delta >= 0 {
        g.add_counters(comet, CounterKind::Loyalty, delta);
    } else {
        g.remove_counters(comet, CounterKind::Loyalty, -delta);
    }
}

/// "0: Roll a six-sided die." The roll is the game's own recorded randomness (CR 706); each result of its table is this
/// card's own branch, and none of them targets: every choice is made as it resolves (CR 608.2d).
fn roll(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let comet = item.object;
    let you = item.controller;
    match g.roll_die(you, 6).result {
        // "[+2], then create two 1/1 green Squirrel creature tokens. They gain haste until end of turn."
        1 | 2 => {
            loyalty(g, comet, 2);
            let squirrel = g.card_id(&SQUIRREL);
            let made = g.create_tokens_together(&[(squirrel, you), (squirrel, you)])?;
            g.end_sentence();
            instruction(g, |g| {
                for token in made {
                    haste_until_end_of_turn(g, comet, token);
                }
            });
        }
        // "[-1], then return a card with mana value 2 or less from your graveyard to your hand."
        3 => {
            loyalty(g, comet, -1);
            let eligible: Vec<ObjectId> =
                g.state().zone(Zone::graveyard(you)).iter().copied().filter(|&c| g.mana_value(c) <= 2).collect();
            let cards = g.state().zone(Zone::graveyard(you)).to_vec();
            if let Some(card) = choose::card_from(g, you, comet, Purpose::PutIntoHand, cards, eligible)? {
                g.return_to_hand(&[card]);
            }
        }
        // "Comet deals damage equal to the number of loyalty counters on him to a creature or player, then [-2]."
        4 | 5 => {
            let mut candidates: Vec<Target> = g
                .state()
                .battlefield()
                .iter()
                .copied()
                .filter(|&id| g.chars(id).is(CardType::Creature))
                .map(Target::Object)
                .collect();
            candidates.extend(g.state().apnap().map(Target::Player));
            let ask = RequestKind::ChooseTarget { label: "", slot_player: None, total: None, source: comet, purpose: Purpose::Targets, candidates, optional: false, least: 1, count: 1, any_number: false };
            if let Answer::Targets(chosen) = g.ask(you, ask)?
                && let Some(&hit) = chosen.first()
            {
                let amount = g.obj(comet).counters(CounterKind::Loyalty).max(0) as u16;
                g.damage(comet, hit, amount);
            }
            loyalty(g, comet, -2);
        }
        // "[+1], and you may activate Comet's loyalty ability two more times this turn."
        _ => {
            loyalty(g, comet, 1);
            g.allow_extra_loyalty_activations(comet, 2);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cantrip_api::loyalty(0), roll, "0: Roll a six-sided die.")],
    ..oracle::COMET_STELLAR_PUP
};

pub static EXTRA: &[&CardDef] = &[&SQUIRREL];
