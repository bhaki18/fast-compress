# fast-compress

a simple file compressor in rust

## punti fondamentali

allora la prima cosa è pensare all'algoritmo di compressione,
questa è la prima volta che ne faccio uno quindi cerco di pensare ad alcune opzioni utili
ma non so quanto possano essere efficaci,ma proviamo

### primo algoritmo

la solita struttura numero-lettura tipo "AAABBBCCCD1100" -> "3A\3B\4C\1d\2|1\2|0"
è facile da scrivere e da capire ma forse troppo poco efficace

### secondo algoritmo
