# turbo-blif

low-memory-usage BLIF (berkeley logic interchange format) parser.

|                                             | this | SIS | yosys | abc | [pip blifparser] | [lorina] | [crates.io blif-parser] |
| ------------------------------------------- | ---- | --- | ----- | --- | ---------------- | -------- | ----------------------- |
| top module, latches, LUTs                   |  x   |  x  |   x   |  x  |         x        |     x    |          x              |
| different latch types                       |  x   |  x  |   x   |  x  |         -        |     x    |          x              |
| usage of library gates                      |  x   |  x  |   x   |  x  |         -        |     -    |          x              |
| empty lines, padding, and comments          |  x   |  x  |   ?   |  ?  |         -        |     x    |          x              |
| quirk 1: allow omit `.end` and `.module`    |  x   |  x  |   ?   |  ?  |         ?        |     ?    |          ?              |
| quirk 2: `\` to continue on next line       |  x   |  x  |   x   |  ?  |         -        |     -    |          x              |
| multiple models per file & sub-circuits     |  x   |  x  |   x   |  ?  |         -        |     -    |          x              |
| model attr: `.clock`                        |  x   |  x  |   x   |  ?  |         -        |     -    |          -              |
| sub-file references                         |  x   |  x  |   ?   |  ?  |         x        |     -    |          -              |
| finite state machines (`.start_kiss`)       |  x   |  x  |   -   |  -  |         x        |     -    |          -              |
| clock constraints (mostly for simulation)   | WIP  |  x  |   -   |  ?  |         -        |     -    |
| delay constraints                           | WIP  |  x  |   -   |  ?  |         -        |     -    |
| full BLIF specification [^1]                |  x   |  x  |   -   |  -  |         -        |     -    |
| ------------------------------------------- | ---- | --- | ----- | --- | ---------------- | -------- |
| abc extension: "Black- & White-boxes" [^2]  | WIP  |  -  |   -   |  x  |         -        |     -    |
| extension: `.blackbox`                      | WIP  |  -  |   x   |  ?  |         -        |     -    |
| yosys extension: `.cname`: cell name attr   |  x   |  -  |   x   |  ?  |         -        |     -    |
| yosys extension: `.attr` and `.param`       |  x   |  -  |   x   |  ?  |         -        |     -    |
| extension: `.barbuff` / `.conn`             |  x   |  -  |   x   |  ?  |         -        |     -    |



[^1]: https://people.eecs.berkeley.edu/~alanmi/publications/other/blif.pdf
[^2]: https://people.eecs.berkeley.edu/~alanmi/publications/other/boxes01.pdf
[pip blifparser]: https://github.com/mario33881/blifparser
[lorina]: https://github.com/hriener/lorina
[crates.io blif-parser]: https://github.com/ucb-bar/blif-parser/

- the latest BLIF specification (dated July 28, 1992)
- all yosys BLIF extensions
  (supports reading of BLIF files generated with `write_blif -iname -iattr -param -cname -blackbox -attr -conn`)
- KISS state machines (which yosys doesn't even support)
- clock and delay constraints (yosys just ignores those)

If you found a program that generates non-standard BLIF attributes or keywords, please open a GitHub issue.
We want to support all non-standard extensions.
