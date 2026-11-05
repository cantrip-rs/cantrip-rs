//! What one seat is told of a game, as plain data (docs/ai-player-plan.md, Phase 1): the observation an AI player
//! reads, built by `cantrip-ai` from the engine and the seat's knowledge, and nothing else. No engine is in it, so it
//! builds for the browser as well, where a policy runs in a player's own tab.
//!
//! Objects are named by their canonical index in `Observation::objects`, never by an engine id: zone by zone in a
//! fixed order, each zone in the order a player sees it (a library, a graveyard, the stack by position; an unordered
//! zone by name and what is visible of each object). Two games that differ only in what the seat cannot know give the
//! same observation, byte for byte.
//!
//! One module per concept: `observation` is the data. The encoding a network reads, and answers factored over
//! canonical indices, will be modules beside it.

pub mod encode;
pub mod factor;
mod observation;

pub use observation::*;

/// The observation as bytes (postcard): what is stored, sent or compared.
pub fn to_bytes(observation: &Observation) -> Vec<u8> {
    postcard::to_allocvec(observation).expect("an observation always serializes")
}

/// An observation from its bytes. `None` for bytes of another schema or none at all.
pub fn from_bytes(bytes: &[u8]) -> Option<Observation> {
    let observation: Observation = postcard::from_bytes(bytes).ok()?;
    (observation.schema == SCHEMA_VERSION).then_some(observation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_payment_observation_preserves_operation_kind_and_objects() {
        let mut observation = sample();
        observation.request.as_mut().unwrap().ask = Some(Ask { may_cancel: true, source: Ref::Object(0),
            kind: AskKind::ChooseCostPayment { choices: vec![CostPaymentObs { label: "Sacrifice a creature".into(),
                objects: vec![Ref::Object(1)], kind: CostPaymentKindObs::Sacrifice }] } });
        assert_eq!(from_bytes(&to_bytes(&observation)), Some(observation.clone()));
        let json = serde_json::to_string(&observation).unwrap();
        assert_eq!(serde_json::from_str::<Observation>(&json).unwrap(), observation);
    }

    pub(crate) fn sample() -> Observation {
        let bear = CharsObs {
            types: 1 << 1,
            supertypes: 0,
            colors: 1 << 4,
            subtypes: vec![7],
            power: Some(2),
            toughness: Some(2),
            keywords: 0,
            mana_value: 2,
        };
        Observation {
            schema: SCHEMA_VERSION,
            mode: InfoMode::Decklist,
            seat: 0,
            turn: TurnObs { number: 3, active: 1, step: 5, priority: 0, starting: 1 },
            players: vec![
                PlayerObs { snow_pool: [0; 6], life: 20, poison: 0, energy: 0, lost: false, lands_played: 0, mulligans: 0, pool: [0, 0, 0, 1, 0, 0], hand: 1, library: 2, monarch: false, speed: None, library_known: vec![] },
                PlayerObs { snow_pool: [0; 6], life: 17, poison: 1, energy: 2, lost: false, lands_played: 1, mulligans: 1, pool: [0; 6], hand: 1, library: 30, monarch: true, speed: Some(2), library_known: vec!["Shock".into()] },
            ],
            objects: vec![
                ObjectObs::Hidden { place: Place { zone: ZoneObs::Library, player: 0 }, owner: 0, position: Some(0), identity: Some("Forest".into()), face_down: false },
                ObjectObs::Hidden { place: Place { zone: ZoneObs::Library, player: 0 }, owner: 0, position: Some(1), identity: None, face_down: false },
                ObjectObs::Visible(Box::new(VisibleObs {
                    place: Place { zone: ZoneObs::Battlefield, player: 0 },
                    position: None,
                    owner: 1,
                    controller: 1,
                    name: Some("Grizzly Bears".into()),
                    chars: bear,
                    face_down: false,
                    token: false,
                    tapped: true,
                    damage: 1,
                    counters: vec![(0, 1)],
                    attacking: Some(Ref::Player(0)),
                    blocked: true,
                    blocking: vec![],
                    sick: false,
                    attached: Ref::None,
                    spell: None,
                })),
                ObjectObs::Ability { position: 0, source: Ref::Object(2), name: "Grizzly Bears".into(), kind: 2, controller: 1, targets: vec![Ref::Object(0), Ref::Player(0)], x: 0 },
            ],
            events: vec![EventObs { turn: 3, kind: 2, player: Some(1), objects: vec![Ref::Object(2), Ref::None], from: Some(Place { zone: ZoneObs::Hand, player: 1 }), to: Some(Place { zone: ZoneObs::Battlefield, player: 0 }), amount: 0 }],
            request: Some(RequestObs { player: 0, kind: "DeclareBlockers".into(), candidates: vec![2], ask: Some(Ask { may_cancel: false, source: Ref::None, kind: AskKind::DeclareBlockers { eligible: vec![Ref::Object(2)], attackers: vec![Ref::Object(3)], allowed: vec![(Ref::Object(2), Ref::Object(3))], required: vec![], most: 0, min_blockers: vec![2], max_blockers: vec![255], max_attackers_per_blocker: vec![1] } }) }),
            decklist: vec![Count { name: "Forest".into(), count: 17 }],
            pools: vec![vec![], vec![Count { name: "Shock".into(), count: 3 }]],
            seen: vec![vec![], vec![Count { name: "Grizzly Bears".into(), count: 1 }]],
        }
    }

    #[test]
    fn semantic_quantity_width_keeps_legacy_encoded_bytes_and_exact_raw_amounts() {
        let mut observation = sample();
        let machine = factor::Machine::new(observation.request.as_ref().unwrap().ask.as_ref().unwrap(), &[]);
        let encoded = encode::to_compact(&encode::encode_machine(&observation, &machine, &|_: &str| 0));
        assert_eq!(encoded[0], 4, "the native cost-operation encoding has its schema-4 header");
        // The historical schema-2 golden checks the same non-snow payload with its original header.
        // Compare in memory; never retag the serialized observation or a stored model.
        let hash = std::iter::once(2u8).chain(encoded.iter().skip(1).copied())
            .fold(0xcbf29ce484222325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100000001b3));
        assert_eq!(encoded.len(), 477);
        assert_eq!(hash, 0xce448f742172302a);
        if let Ok(folder) = std::env::var("CANTRIP_DAMAGE_PROOF") {
            std::fs::write(std::path::Path::new(&folder).join(format!("legacy-encoded-{}.bin", SCHEMA_VERSION)), &encoded).unwrap();
        }
        observation.events[0].amount = i64::from(u32::MAX) + 7;
        observation.request.as_mut().unwrap().ask.as_mut().unwrap().kind = AskKind::ChooseReplacement {
            subject: Ref::None, life: Some(-4294967295), sources: vec![], hints: vec![], damage: Some((Ref::Object(0), Ref::Player(1), i64::from(u32::MAX) + 7)), turn: false, counters: vec![(0, i64::from(u32::MAX) + 8)], draw: false,
        };
        assert_eq!(from_bytes(&to_bytes(&observation)), Some(observation.clone()));
        let json = serde_json::to_string(&observation).unwrap();
        assert_eq!(serde_json::from_str::<Observation>(&json).unwrap(), observation);
        let AskKind::ChooseReplacement { .. } = &observation.request.as_ref().unwrap().ask.as_ref().unwrap().kind else { unreachable!() };
        let (_, numbers) = encode::ask_numbers(&observation.request.as_ref().unwrap().ask.as_ref().unwrap().kind);
        assert_eq!(numbers[4], 4294967295);
    }

    #[test]
    fn round_trips_through_postcard() {
        let observation = sample();
        let bytes = to_bytes(&observation);
        assert_eq!(from_bytes(&bytes), Some(observation.clone()));
        assert_eq!(to_bytes(&from_bytes(&bytes).unwrap()), bytes);
    }

    #[test]
    fn high_keyword_bits_round_trip_exactly_in_postcard_and_json() {
        let mut observation = sample();
        let ObjectObs::Visible(object) = &mut observation.objects[2] else { panic!("visible sample object") };
        object.chars.keywords = (1u128 << 64) | (1u128 << 66) | (1u128 << 35);
        assert_eq!(from_bytes(&to_bytes(&observation)), Some(observation.clone()));
        let json = serde_json::to_string(&observation).unwrap();
        assert_eq!(serde_json::from_str::<Observation>(&json).unwrap(), observation);
    }

    #[test]
    fn refuses_another_schema() {
        let mut observation = sample();
        // Schema 4 used private counter IDs that collide with the published counter table.
        // The bytes still deserialize, so the version check must reject their old meaning.
        observation.schema = 4;
        assert_eq!(from_bytes(&to_bytes(&observation)), None);
        observation.schema = SCHEMA_VERSION + 1;
        assert_eq!(from_bytes(&to_bytes(&observation)), None);
        assert_eq!(from_bytes(&[]), None);
    }
}
