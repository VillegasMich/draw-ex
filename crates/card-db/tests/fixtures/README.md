# YGOPRODeck fixtures

Recorded once from the live API (2026-11-15) with the `draw-ex/0.1.0` User-Agent, then
trimmed. Tests serve them with `wiremock`; nothing here is fetched at test time.

- `checkDBVer.json`: `checkDBVer.php`.
- `cardinfo_en.json`: `cardinfo.php?id=83764719,14558127,55144522` plus
  `cardinfo.php?id=46986414,1861629` (Dark Magician, and Decode Talker as a Link
  monster), merged.
- `cardinfo_fr.json`: `cardinfo.php?id=83764719,14558127,55144522&language=fr`.

Edits after recording:

- `card_sets` and `card_prices` are removed from every card, to keep the files small.
- Pot of Greed (`55144522`) is removed from the French file, to stand in for a card
  without a translation.
- Dark Magician has a second `card_images` entry, `36996508`, to stand in for an
  alternate artwork. The live API returned no alternate artworks when these were
  recorded.
