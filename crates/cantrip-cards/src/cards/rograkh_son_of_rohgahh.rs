use cantrip_api::*;
use cantrip_oracle as oracle;

// Characteristics only: "Partner" (CR 702.124a) is a keyword ability that modifies the rules for deck construction in the
// Commander variant and functions before the game begins; it does nothing in play. Which two cards may be a deck's commanders
// together is the deck rules' (`Cantrip.Decks.DeckRules`, which reads the Oracle text), and the engine has no command zone for
// commanders. First strike, menace and trample are core keywords the generator reads from the card's JSON.
pub static CARD: CardDef = CardDef { ..oracle::ROGRAKH_SON_OF_ROHGAHH };
