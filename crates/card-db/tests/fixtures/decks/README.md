# Deck list fixtures

Written by hand from the `.ydk` format as YGOPRODeck and EDOPro export it, not
downloaded. The passcodes are real cards; the decks are not meant to be playable.

- `ygoprodeck.ydk`: YGOPRODeck style. `#created by YGOPRODeck`, then `#main`, `#extra`
  and `!side`; LF line endings; some passcodes zero-padded to 8 digits (`08267140`,
  `04280258`). 20 main, 5 extra, 5 side, 21 distinct passcodes.
- `edopro.ydk`: EDOPro style. `#created by Player`; **CRLF** line endings, a trailing
  space after one passcode and a blank line in the main deck; passcodes unpadded
  (`8267140`, `4280258`). 9 main, 2 extra, 3 side, 10 distinct passcodes.

`.gitattributes` here marks `*.ydk` as `-text`, so Git keeps the CRLF in `edopro.ydk`
despite the repo-wide `eol=lf`.
