# turbo-blif

low-memory-usage BLIF (berkeley logic interchange format) parser.

## Comparison with other parsers
|                                             | this | SIS | yosys | abc | [pip blifparser] | [lorina] | [crates.io blif-parser] | [quaigh] | [libblifparse] | [spydrnet] |
| ------------------------------------------- | ---- | --- | ----- | --- | ---------------- | -------- | ----------------------- | -------- | -------------- | ---------- |
| top module, latches, LUTs                   |  x   |  x  |   x   |  x  |         x        |     x    |          x              |    x     |        x       |     x      |
| different latch types                       |  x   |  x  |   x   |  x  |         -        |     x    |          x              |    -     |        x       |     -      |
| usage of library gates / latches            |  x   |  x  |   x   |  x  |         -        |     -    |          x              |    -     |        -       |     ~      |
| empty lines, padding, and comments          |  x   |  x  |   ?   |  x  |         -        |     x    |          x              |    x     |        x       |     x      |
| quirk 1: allow omit `.end` and `.model`     |  x   |  x  |   ?   |  -  |         ?        |     ?    |          ?              |    -     |        -       |     ?      |
| 'quirk' 2: `\` to continue on next line     |  x   |  x  |   x   |  x  |         -        |     -    |          x              |    x     |        x       |     x      |
| multiple models per file & sub-circuits     |  x   |  x  |   x   |  -  |         -        |     -    |          x              |    -     |        x       |     x      |
| model attr: `.clock`                        |  x   |  x  |   x   |  -  |         -        |     -    |          -              |    -     |        -       |     x      |
| sub-file references                         |  x   |  x  |   ?   |  -  |         x        |     -    |          -              |    -     |        -       |     -      |
| finite state machines (`.start_kiss`)       |  x   |  x  |   -   |  -  |         x        |     -    |          -              |    -     |        -       |     -      |
| clock constraints (mostly for simulation)   |  x   |  x  |   -   |  -  |         -        |     -    |          -              |    -     |        -       |     -      |
| delay constraints                           | soon |  x  |   -   |  ~  |         -        |     -    |          -              |    -     |        -       |     -      |
| full BLIF specification [^1]                | soon |  x  |   -   |  -  |         -        |     -    |          -              |    -     |        -       |     -      |
| extension: "Black- & White-boxes" [^2]      | soon |  -  |   -   |  -  |         -        |     -    |          -              |    -     |        -       |     -      |
| extension: `.blackbox`                      | soon |  -  |   x   |  x  |         -        |     -    |          -              |    -     |        x       |     x      |
| extension: `.cname` (EBLIF[^3])             |  x   |  -  |   x   |  -  |         -        |     -    |          -              |    -     |        x       |     x      |
| extension: `.attr` and `.param` (EBLIF[^3]) |  x   |  -  |   x   |  -  |         -        |     -    |          -              |    -     |        x       |     x      |
| extension: `.conn` (EBLIF[^3])              |  x   |  -  |   x   |  -  |         -        |     -    |          -              |    -     |        x       |     x      |
| extension: `.barbuff` (identical: `.conn`)  |  x   |  -  |   x   |  -  |         -        |     -    |          -              |    -     |        -       |     -      |
| extension: `.and_gate_delay`                | soon |  -  |   -   |  x  |         -        |     -    |          -              |    -     |        -       |     -      |


[^1]: https://people.eecs.berkeley.edu/~alanmi/publications/other/blif.pdf
[^2]: https://people.eecs.berkeley.edu/~alanmi/publications/other/boxes01.pdf
[^3]: https://docs.verilogtorouting.org/en/latest/vpr/file_formats/

[pip blifparser]: https://github.com/mario33881/blifparser
[lorina]: https://github.com/hriener/lorina
[crates.io blif-parser]: https://github.com/ucb-bar/blif-parser/
[quaigh]: https://github.com/Coloquinte/quaigh/
[libblifparse]: https://github.com/verilog-to-routing/libblifparse
[spydrnet]: https://github.com/byuccl/spydrnet


## Tested with BLIF generators
- yosys `write_blif`, including all supported extensions: `write_blif -iname -iattr -param -cname -blackbox -attr -conn`
- abc write blif

## Goals
- parse every BLIF file in existence

If you found a program that generates or consists of non-standard BLIF attributes or keywords, please open a GitHub issue.
We want to support all non-standard extensions.
