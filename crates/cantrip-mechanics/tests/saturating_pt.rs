use cantrip_api::*;
use cantrip_api::history::History;
use cantrip_mechanics::statics::plus_pt_saturating;

struct Unused;
impl LayerQuery for Unused {
    fn state(&self) -> &GameState { unreachable!() }
    fn cards(&self) -> &CardTable { unreachable!() }
    fn history(&self) -> History<'_> { unreachable!() }
    fn chars(&self, _: ObjectId) -> Chars { unreachable!() }
    fn chars_with_defined_pt(&self, _: ObjectId) -> Chars { unreachable!() }
}

fn chars(power: Option<i16>, toughness: Option<i16>) -> Chars {
    Chars { ability_origins: [ObjectId::NONE; MAX_ABILITIES],
        card: CardId(0), name: "", types: TypeSet::NONE, supertypes: Supertypes::default(),
        subtypes: SubtypeSet::NONE, colors: ColorSet::NONE, power, toughness, base_power: power, keywords: Keywords::NONE,
        protection: ColorSet::NONE, protection_colorless: false, protection_players: 0,
        protection_types: TypeSet::NONE, protection_subtypes: SubtypeSet::NONE, protection_mana_value: None,
        spell: false, protection_colored_spells: false, protection_everything: false,
        hexproof_from: HexproofFrom::NONE, must_attack: 0, toxic: 0, abilities: [None; MAX_ABILITIES],
        no_cost: false, name_set: false, lost_all: false, text_lost: false, riot: 0,
    }
}

#[test]
fn modifier_preserves_boundaries_and_absent_characteristics() {
    let mut high = chars(Some(i16::MAX), Some(i16::MAX));
    plus_pt_saturating::<1, 1>(&Unused, ObjectId(0), &mut high);
    assert_eq!((high.power, high.toughness), (Some(i16::MAX), Some(i16::MAX)));
    let mut low = chars(Some(i16::MIN), Some(i16::MIN));
    plus_pt_saturating::<-1, -1>(&Unused, ObjectId(0), &mut low);
    assert_eq!((low.power, low.toughness), (Some(i16::MIN), Some(i16::MIN)));
    let mut absent = chars(None, None);
    plus_pt_saturating::<1, 0>(&Unused, ObjectId(0), &mut absent);
    assert_eq!((absent.power, absent.toughness), (None, None));
}
