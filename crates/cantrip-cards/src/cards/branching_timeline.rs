use cantrip_api::*;
use cantrip_mechanics::{lands, library, targets, top_of_library};
use cantrip_oracle as oracle;

/// Reveal the top X cards (CR 701.20a), where X is the number of differently named lands you control. Put every
/// permanent card among them onto the battlefield tapped together (CR 608.2f, 110.2a); the other cards go to the
/// bottom in a random order.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let revealed = top_of_library(g, player, lands::differently_named_lands(g, player));
    if revealed.is_empty() {
        return Ok(());
    }

    g.reveal(player, &revealed);
    let entering: Vec<(ObjectId, PlayerId, bool)> = revealed
        .iter()
        .copied()
        .filter(|&card| targets::is_permanent_card(&g.chars(card)))
        .map(|card| (card, player, true))
        .collect();
    if !entering.is_empty() {
        g.put_all_onto_battlefield_as(&entering)?;
    }
    library::rest_to_bottom_in_random_order(g, player, &revealed);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ..oracle::BRANCHING_TIMELINE
};

#[cfg(test)]
mod tests {
    use super::*;

    const YOU: PlayerId = PlayerId(0);
    static DOOR_A: CardDef = CardDef {
        name: "A",
        types: TypeSet::of(&[CardType::Enchantment]),
        subtypes: SubtypeSet::of(&[Subtype::Room]),
        ..CardDef::BLANK
    };
    static DOOR_B: CardDef = CardDef {
        name: "B",
        ..DOOR_A
    };
    static ROOM: CardDef = CardDef {
        name: "A // B",
        split: Some((&DOOR_A, &DOOR_B)),
        ..DOOR_A
    };

    struct QueryWithRoomLands {
        state: GameState,
    }

    impl Query for QueryWithRoomLands {
        fn state(&self) -> &GameState {
            &self.state
        }

        fn card(&self, _: CardId) -> &'static CardDef {
            &ROOM
        }

        fn chars(&self, id: ObjectId) -> Chars {
            let name = match self.state.obj(id).marks.doors {
                0 => "",
                1 => "A",
                2 => "B",
                _ => "A // B",
            };
            Chars {
                card: CardId(0),
                name,
                types: TypeSet::of(&[CardType::Land]),
                supertypes: Supertypes::default(),
                subtypes: SubtypeSet::NONE,
                colors: ColorSet::NONE,
                power: None,
                toughness: None,
                keywords: Keywords::NONE,
                protection: ColorSet::NONE,
                protection_colorless: false,
                protection_players: 0,
                protection_types: TypeSet::NONE,
                protection_subtypes: SubtypeSet::NONE,
                protection_mana_value: None,
                spell: false,
                protection_colored_spells: false,
                protection_everything: false,
                hexproof_from: HexproofFrom::NONE,
                must_attack: 0,
                toxic: 0,
                abilities: [None; MAX_ABILITIES],
                no_cost: false,
                name_set: false,
                lost_all: false,
                text_lost: false,
            }
        }

        fn token_chars(&self, _: CardId, _: Option<Copiable>) -> Chars {
            self.chars(ObjectId(0))
        }

        fn lost_all_abilities(&self, _: ObjectId) -> bool {
            false
        }

        fn card_id(&self, _: &'static CardDef) -> CardId {
            CardId(0)
        }

        fn linked_card(&self, card: CardId) -> CardId {
            card
        }

        fn text_linked(&self, source: ObjectId) -> Linked {
            Linked {
                object: source,
                text: CardId(0),
                acquired: Acquired::OWN,
            }
        }

        fn events(&self) -> &[Record] {
            &[]
        }

        fn can_target(&self, _: &Asker, _: Target) -> bool {
            false
        }
    }

    fn room_lands(doors: &[u8]) -> QueryWithRoomLands {
        let mut state = GameState::new(2, 0);
        for &unlocked in doors {
            let room = state.push_obj(Obj::new(CardId(0), YOU, Zone::BATTLEFIELD));
            state.zone_push(Zone::BATTLEFIELD, room);
            state.update_obj(room, |obj| obj.marks.doors = unlocked);
        }
        QueryWithRoomLands { state }
    }

    /// A Room with doors A and B coexists with the separate A and B doors; unnamed Rooms are excluded (CR 201.2b).
    #[test]
    fn counts_the_largest_room_name_set() {
        let query = room_lands(&[DOOR_LEFT | DOOR_RIGHT, DOOR_LEFT, DOOR_RIGHT, 0]);
        assert_eq!(lands::differently_named_lands(&query, YOU), 2);

        let query = room_lands(&[DOOR_LEFT | DOOR_RIGHT, DOOR_LEFT, 0]);
        assert_eq!(lands::differently_named_lands(&query, YOU), 1);
    }
}
