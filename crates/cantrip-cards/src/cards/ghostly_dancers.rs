use cantrip_api::*;
use cantrip_mechanics::select::{Count, Selection};
use cantrip_mechanics::{choose, effects, tokens, triggers};
use cantrip_oracle as oracle;

/// The 3/1 white Spirit creature token with flying that eerie makes.
static SPIRIT: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    text: "Flying",
    ..token!(Spirit)
};

/// "Return an enchantment card from your graveyard to your hand or unlock a locked door of a Room you control": each option
/// is offered only if it can be done (CR 608.2d, 101.3), the one that can be done being taken without asking. The Room is
/// chosen as the ability resolves, one the controller has with a door still locked, and then the locked door to unlock, which is
/// asked only when both are locked (CR 709.5f, 709.5d).
fn return_an_enchantment_or_unlock_a_door(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let enchantments: Vec<ObjectId> = g.state().zone(Zone::graveyard(you)).iter().copied().filter(|&id| g.chars(id).is(CardType::Enchantment)).collect();
    let room = g.words(item.object).subtype(Subtype::Room);
    let rooms: Vec<ObjectId> = g.permanents_of(you).into_iter().filter(|&id| g.chars(id).has_subtype(room) && g.obj(id).marks.doors != DOOR_LEFT | DOOR_RIGHT).collect();
    let mut options = Vec::new();
    if !enchantments.is_empty() {
        options.push(0u8);
    }
    if !rooms.is_empty() {
        options.push(1u8);
    }
    let picked = match options.as_slice() {
        [] => return Ok(()),
        [only] => *only,
        _ => {
            let card = g.obj(item.object).card;
            options[choose::one_of(g, you, item.object, options.iter().map(|&option| Label::Card(card, option)).collect())? as usize]
        }
    };
    if picked == 0 {
        let chosen = choose::select(g, you, item.object, Purpose::PutIntoHand, enchantments, Selection::of(Count::AsManyAsCan(1), &[]))?;
        g.return_to_hand(&chosen);
    } else {
        let chosen = choose::select(g, you, item.object, Purpose::Choose, rooms, Selection::of(Count::AsManyAsCan(1), &[]))?;
        for permanent in chosen {
            effects::unlock_a_locked_door(g, you, item.object, permanent)?;
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            return_an_enchantment_or_unlock_a_door,
            "When this creature enters, return an enchantment card from your graveyard to your hand or unlock a locked door of a Room you control.",
        ),
        triggered(
            triggers::EERIE,
            |g, item| {
                tokens::create(g, item.controller, &SPIRIT, 1)?;
                Ok(())
            },
            "Eerie \u{2014} Whenever an enchantment you control enters and whenever you fully unlock a Room, create a 3/1 white Spirit creature token with flying.",
        ),
    ],
    labels: &["Return an enchantment card from your graveyard to your hand", "Unlock a locked door of a Room you control"],
    ..oracle::GHOSTLY_DANCERS
};

pub static EXTRA: &[&CardDef] = &[&SPIRIT];
