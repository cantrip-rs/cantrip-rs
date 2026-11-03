use cantrip_api::*;
use cantrip_mechanics::copy;
use cantrip_oracle as oracle;

/// Copy any artifact as this enters, adding enchantment to the copied values (CR 614.1c, 707.9b).
fn enter_as_copy(g: &mut dyn Game, entry: &mut Entering) -> Result<(), Suspend> {
    copy::enter_as_copy(g, entry, |chars| chars.is(CardType::Artifact), |_, values| values.with_type(CardType::Enchantment))
}

pub static CARD: CardDef = CardDef {
    enters_as_copy: Some(enter_as_copy),
    ..oracle::COPY_ARTIFACT
};
