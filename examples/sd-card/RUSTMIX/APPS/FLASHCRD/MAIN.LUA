-- Rustmix Flashcards SD app metadata script
-- Firmware renders image cards from TOPICS/<TOPIC>/CARDS.TXT records.
-- Image row format: IMG|IMG/<CARD>.X4B|IMG/<CARD>.X4B|topic
-- X4B1: 480x800 1bpp, MSB-first, black pixels as 1 bits.
return { id = "FLASHCRD", version = "0.2.0", image_format = "X4B1" }
